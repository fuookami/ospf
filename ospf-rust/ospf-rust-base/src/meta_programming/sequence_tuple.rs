trait SequenceTuple {
    const LEN: usize;
    type LastSequence: SequenceTuple;
    type Tail;
}

impl SequenceTuple for () {
    const LEN: usize = 0;
    type LastSequence = ();
    type Tail = ();
}

struct SequenceTuplePush<Lhs: SequenceTuple, Rhs> {
    _marker: std::marker::PhantomData<(Lhs, Rhs)>,
}
impl<Lhs: SequenceTuple, Rhs> SequenceTuple for SequenceTuplePush<Lhs, Rhs> {
    const LEN: usize = Lhs::LEN + 1;
    type LastSequence = Lhs;
    type Tail = Rhs;
}
