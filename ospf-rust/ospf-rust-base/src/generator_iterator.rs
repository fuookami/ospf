use std::ops::{Coroutine, CoroutineState};
use std::pin::Pin;

pub struct GeneratorIterator<G>(pub G);

impl<G: Coroutine + Unpin> Iterator for GeneratorIterator<G> {
    type Item = G::Yield;

    fn next(&mut self) -> Option<Self::Item> {
        match Pin::new(&mut self.0).resume(()) {
            CoroutineState::Yielded(x) => Some(x),
            CoroutineState::Complete(_) => None,
        }
    }
}
