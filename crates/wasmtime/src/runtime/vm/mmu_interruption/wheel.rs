//! An [`MmuInterrupter`] implemented as a timing wheel

use super::MmuInterrupter;
use crate::prelude::*;
use crate::runtime::vm::SendSyncPtr;
use core::ffi::c_void;
use core::mem::take;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use rustix::mm::{MapFlags, MprotectFlags, ProtFlags, mmap_anonymous, mprotect, munmap};
use rustix::param::page_size;
use std::sync::{Arc, Mutex, Weak};

/// A timing wheel that functions as an [`MmuInterrupter`]
///
/// It is driven by the embedder calling [`TimingWheelInterrupter::tick`]. Thus,
/// any relationship with wall-clock time is established by the embedder.
///
/// Stores that start running between the same two ticks share an interrupt
/// page, so one `mprotect` interrupts all of them. Thundering-herd concerns are
/// mitigated because actual interruptions are smeared out over time as
/// Cranelift-generated code checks for them. Additionally, the herds are small,
/// since they consist of running fibers only, which are bounded by the number
/// of cores. Herd size would average out to be ≤ cores / slots.len().
pub struct TimingWheelInterrupter {
    /// Number of ticks so far. Wraps in the extreme case. This most often
    /// points to the previous slot; `tick()` advances it before doing anything.
    hand: AtomicU64,
    /// "Hours" on the timing wheel. Each slot contains one page, which gets
    /// protected when the hand reaches the slot. Until then, the page,
    /// unprotected, is doled out to stores which ask to have an interrupt
    /// scheduled at that slot. If there is no page there, one is reclaimed from
    /// `unprotected` or, failing that, newly allocated. A refcount is kept of
    /// stores holding onto any given page.
    slots: Box<[Mutex<Option<Arc<Page>>>]>,
    /// Pages which the hand has reached and whose last holding store has let
    /// go. These are ready to get unprotected (lazily) and then moved to
    /// `unprotected`.
    to_unprotect: Arc<UnprotectQueue>,
    /// Unprotected pages with no holders, ready to be used in `slots` as
    /// necessary
    unprotected: Mutex<Vec<Arc<Page>>>,
    /// Lock to serialize ticks so no page is unprotected before it is protected
    tick_lock: Mutex<()>,
}

impl TimingWheelInterrupter {
    /// Returns an interrupter that gives each store at least
    /// `ticks_per_interrupt` full tick intervals before interrupting it.
    pub fn new(ticks_per_interrupt: u32) -> Self {
        // The +1 makes this "at least N ticks". (Without, it would mean "at
        // most N ticks".) The +1 compensates for the fraction of the current
        // tick that passed before a store acquired its page.
        let slots = usize::try_from(ticks_per_interrupt).unwrap() + 1;
        Self {
            hand: AtomicU64::new(0),
            slots: (0..slots).map(|_| Mutex::new(None)).collect(),
            to_unprotect: Arc::default(),
            unprotected: Mutex::default(),
            tick_lock: Mutex::default(),
        }
    }

    /// Advances the wheel, interrupting stores watching a page at the next
    /// slot.
    ///
    /// Until this is first called, nothing is interrupted.
    pub fn tick(&self) {
        let _lock = self.tick_lock.lock().unwrap();
        let hand = self.hand.fetch_add(1, Ordering::AcqRel).wrapping_add(1);
        let page_to_protect = {
            let mut slot = self.slot(hand).lock().unwrap();
            match slot.as_ref() {
                None => None,
                Some(page) => {
                    // Set the REACHED bit and then see if the refcount turned
                    // out to be zero, in which case we'll walk that back.
                    if page
                        .reached_and_refcount
                        .fetch_or(REACHED, Ordering::AcqRel)
                        & REFCOUNT_MASK
                        == 0
                    {
                        // No stores were watching this page, so there's no
                        // sense protecting it. Walk back our reach-marking of
                        // it.
                        //
                        // No one can increment since the above ==0 check
                        // because all other writers to `reached_and_refcount`
                        // either hold a slot lock, hold the tick lock, or are
                        // subtracts.
                        page.reached_and_refcount.store(0, Ordering::Release);
                        None
                    } else {
                        // This page is burned now; it has stores watching it,
                        // and we're about to protect it. Don't hand it out to
                        // any new stores, at least not until it's been
                        // renounced by all of them and recycled.
                        //
                        // We drop our reference, but those stores are still
                        // hanging on. When the last one releases it, we'll put
                        // it into `to_unprotect`.
                        slot.take()
                    }
                }
            }
        };
        // Keep this expensive part outside the slot lock.
        if let Some(page) = page_to_protect {
            page.protect();
        }

        let drained = take(&mut *self.to_unprotect.lock().unwrap());
        for page in drained {
            page.unprotect();
            page.reached_and_refcount.store(0, Ordering::Release);
            self.unprotected.lock().unwrap().push(page);
        }
    }

    fn slot(&self, hand: u64) -> &Mutex<Option<Arc<Page>>> {
        let len = u64::try_from(self.slots.len()).unwrap();
        &self.slots[usize::try_from(hand % len).unwrap()]
    }
}

impl MmuInterrupter for TimingWheelInterrupter {
    /// Hands out a page from the furthest-in-the-future slot.
    fn acquire_page(&self) -> Box<dyn super::PageHandle> {
        let hand = self.hand.load(Ordering::Acquire);
        let mut slot = self.slot(hand).lock().unwrap();
        let page = slot.get_or_insert_with(|| {
            self.unprotected
                .lock()
                .unwrap()
                .pop()
                .unwrap_or_else(|| Page::new(&self.to_unprotect))
        });
        let prev = page.reached_and_refcount.fetch_add(1, Ordering::AcqRel);
        assert_ne!(
            prev & REFCOUNT_MASK,
            REFCOUNT_MASK,
            "too many stores share an interrupt page"
        );
        Box::new(PageHandle(page.clone()))
    }
}

struct PageHandle(Arc<Page>);

impl super::PageHandle for PageHandle {
    fn page_ptr(&self) -> NonNull<c_void> {
        self.0.ptr.as_non_null()
    }
}

impl Drop for PageHandle {
    fn drop(&mut self) {
        let page = &self.0;
        // A failed upgrade() is okay: if the interrupter is gone, so is every
        // Engine that could load from the page, so it needn't be unprotected.
        if page.reached_and_refcount.fetch_sub(1, Ordering::AcqRel) == REACHED + 1
            && let Some(to_unprotect) = page.to_unprotect.upgrade()
        {
            to_unprotect.lock().unwrap().push(page.clone());
        }
    }
}

type UnprotectQueue = Mutex<Vec<Arc<Page>>>;

/// A bit of `Page::reached_and_refcount` which marks a `Page` as having been
/// reached by the hand of the timing wheel and severed, reference-wise, from
/// the wheel. Its purpose is to disclaim the wheel's hold on it to
/// `PageHandle`'s drop impl so the last dropper of it can queue it for
/// recycling. Otherwise, we'd have to do something less efficient there like
/// identifying and locking the applicable slot and making sure the page isn't
/// there.
const REACHED: u32 = 1 << 31;
/// Masks the number of stores holding a page out of `Page::reached_and_refcount`
const REFCOUNT_MASK: u32 = REACHED - 1;

struct Page {
    ptr: SendSyncPtr<c_void>,
    /// `REACHED` plus the number of stores holding this page. They share one
    /// atomic so `tick()` can mark the page reached and read its refcount in a
    /// single step; otherwise, a concurrent last release could miss the REACHED
    /// bit and never queue the page for unprotecting.
    reached_and_refcount: AtomicU32,
    /// Weak, since the queue holds pages
    to_unprotect: Weak<UnprotectQueue>,
}

impl Page {
    fn new(to_unprotect: &Arc<UnprotectQueue>) -> Arc<Page> {
        // SAFETY: `mmap_anonymous()` is unsafe, but passing a null ptr, as we
        // do, satisfies its safety conditions, letting the kernel select the
        // address.
        let ptr = unsafe {
            mmap_anonymous(
                ptr::null_mut(),
                page_size(),
                ProtFlags::READ,
                MapFlags::PRIVATE,
            )
        }
        .expect("an interrupt page should be allocable");
        Arc::new(Page {
            ptr: SendSyncPtr::new(
                NonNull::new(ptr).expect("a successful mmap should not return null"),
            ),
            reached_and_refcount: AtomicU32::new(0),
            to_unprotect: Arc::downgrade(to_unprotect),
        })
    }

    fn mprotect(&self, flags: MprotectFlags) {
        // SAFETY: The page is mapped for as long as `self` lives.
        unsafe { mprotect(self.ptr.as_ptr(), page_size(), flags) }
            .expect("an interrupt page should be protectable");
    }

    fn protect(&self) {
        self.mprotect(MprotectFlags::empty());
    }

    fn unprotect(&self) {
        self.mprotect(MprotectFlags::READ);
    }
}

impl Drop for Page {
    fn drop(&mut self) {
        // SAFETY: A page is dropped only once neither the interrupter nor any
        // `PageHandle` holds it, so no Engine can have Wasm load from it.
        unsafe { munmap(self.ptr.as_ptr(), page_size()) }
            .expect("an interrupt page should be unmappable");
    }
}
