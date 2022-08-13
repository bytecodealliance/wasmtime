use crate::fact::core_types::CoreTypes;
use crate::MemoryIndex;
use serde::{Deserialize, Serialize};
use wasm_encoder::{EntityType, ValType};

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct Transcoder {
    pub from_memory: MemoryIndex,
    pub from_memory64: bool,
    pub to_memory: MemoryIndex,
    pub to_memory64: bool,
    pub op: Transcode,
}

/// Possible transcoding operations that must be provided by the host.
///
/// Note that each transcoding operation may have a unique signature depending
/// on the precise operation.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum Transcode {
    Copy(FixedEncoding),
    Latin1ToUtf16,
    Latin1ToUtf8,
    Utf16ToCompactProbablyUtf16,
    Utf16ToCompactUtf16,
    Utf16ToLatin1,
    Utf16ToUtf8,
    Utf8ToCompactUtf16,
    Utf8ToLatin1,
    Utf8ToUtf16,
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum FixedEncoding {
    Utf8,
    Utf16,
    Latin1,
}

impl Transcoder {
    pub fn name(&self) -> String {
        format!(
            "{} (mem{} => mem{})",
            self.op.desc(),
            self.from_memory.as_u32(),
            self.to_memory.as_u32(),
        )
    }

    pub fn ty(&self, types: &mut CoreTypes) -> EntityType {
        let from_ptr = if self.from_memory64 {
            ValType::I64
        } else {
            ValType::I32
        };
        let to_ptr = if self.to_memory64 {
            ValType::I64
        } else {
            ValType::I32
        };

        let ty = match self.op {
            // These direct transcodings take the source pointer, the source
            // code units, and the destination pointer.
            //
            // The memories being copied between are part of each intrinsic and
            // the destination code units are the same as the source.
            // Note that the pointers are dynamically guaranteed to be aligned
            // and in-bounds for the code units length as defined by the string
            // encoding.
            Transcode::Copy(_) | Transcode::Latin1ToUtf16 => {
                types.function(&[from_ptr, from_ptr, to_ptr], &[])
            }

            // Transcoding from utf8 to utf16 takes the from ptr/len as well as
            // a destination. The destination is valid for len*2 bytes. The
            // return value is how many code units were written to the
            // destination.
            Transcode::Utf8ToUtf16 => types.function(&[from_ptr, from_ptr, to_ptr], &[to_ptr]),

            // Transcoding to utf8 as a smaller format takes all the parameters
            // and returns the amount of space consumed in the src/destination
            Transcode::Utf16ToUtf8 | Transcode::Latin1ToUtf8 => {
                types.function(&[from_ptr, from_ptr, to_ptr, to_ptr], &[from_ptr, to_ptr])
            }

            // The return type is a tagged length which indicates which was
            // used
            Transcode::Utf16ToCompactProbablyUtf16 => {
                types.function(&[from_ptr, from_ptr, to_ptr], &[to_ptr])
            }

            // The initial step of transcoding from a fixed format to a compact
            // format. Takes the ptr/len of the source the the destination
            // pointer. The destination length is implicitly the same. Returns
            // how many code units were consumed in the source, which is also
            // how many bytes were written to the destination.
            Transcode::Utf8ToLatin1 | Transcode::Utf16ToLatin1 => {
                types.function(&[from_ptr, from_ptr, to_ptr], &[from_ptr, to_ptr])
            }

            // The final step of transcoding to a compact format when the fixed
            // transcode has failed. This takes the ptr/len of the source that's
            // remaining to transcode. Then this takes the destination ptr/len
            // as well as the destination bytes written so far with latin1.
            // Finally this returns the number of code units written to the
            // destination.
            Transcode::Utf8ToCompactUtf16 | Transcode::Utf16ToCompactUtf16 => {
                types.function(&[from_ptr, from_ptr, to_ptr, to_ptr, to_ptr], &[to_ptr])
            }
        };
        EntityType::Function(ty)
    }
}

impl Transcode {
    /// Returns a human-readable description for this transcoding operation.
    pub fn desc(&self) -> &'static str {
        match self {
            Transcode::Copy(FixedEncoding::Utf8) => "utf8-to-utf8",
            Transcode::Copy(FixedEncoding::Utf16) => "utf16-to-utf16",
            Transcode::Copy(FixedEncoding::Latin1) => "latin1-to-latin1",
            Transcode::Latin1ToUtf16 => "latin1-to-utf16",
            Transcode::Latin1ToUtf8 => "latin1-to-utf8",
            Transcode::Utf16ToCompactProbablyUtf16 => "utf16-to-compact-probably-utf16",
            Transcode::Utf16ToCompactUtf16 => "utf16-to-compact-utf16",
            Transcode::Utf16ToLatin1 => "utf16-to-latin1",
            Transcode::Utf16ToUtf8 => "utf16-to-utf8",
            Transcode::Utf8ToCompactUtf16 => "utf8-to-compact-utf16",
            Transcode::Utf8ToLatin1 => "utf8-to-latin1",
            Transcode::Utf8ToUtf16 => "utf8-to-utf16",
        }
    }
}

impl FixedEncoding {
    pub(crate) fn width(&self) -> u8 {
        match self {
            FixedEncoding::Utf8 => 1,
            FixedEncoding::Utf16 => 2,
            FixedEncoding::Latin1 => 1,
        }
    }
}
