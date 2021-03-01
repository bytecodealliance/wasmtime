/// An extension trait for getting the top element on a stack with an implicit
/// unwrap for if the stack is empty.
pub(crate) trait StackExt<T> {
    fn top(&self) -> &T;
    fn top_mut(&mut self) -> &mut T;
}

impl<T> StackExt<T> for Vec<T> {
    fn top(&self) -> &T {
        self.last().unwrap()
    }

    fn top_mut(&mut self) -> &mut T {
        self.last_mut().unwrap()
    }
}
