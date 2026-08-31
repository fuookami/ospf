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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generator_iterator() {
        let mut generator = GeneratorIterator(#[coroutine] || {
            yield 1;
            yield 2;
            yield 3;
        });

        assert_eq!(generator.next(), Some(1));
        assert_eq!(generator.next(), Some(2));
        assert_eq!(generator.next(), Some(3));
        assert_eq!(generator.next(), None);
    }
}
