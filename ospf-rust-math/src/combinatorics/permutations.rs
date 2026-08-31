use ospf_rust_base::GeneratorIterator;

pub fn permute<T>(input: &[T]) -> Vec<Vec<&T>> {
    let mut a = input.iter().map(|x| x).collect::<Vec<_>>();
    let mut p = input.iter().map(|x| 0).collect::<Vec<_>>();

    let mut perms = Vec::new();
    perms.push(a.clone());

    let mut i = 1;
    while i < input.len() {
        if p[i] < i {
            let j = i % 2 + p[i];
            a.swap(i, j);
            perms.push(a.clone());
            p[i] += 1;
            i = 1;
        } else {
            p[i] = 0;
            i += 1;
        }
    }

    perms
}

pub fn permute_async<T>(input: &[T]) -> impl Iterator<Item = Vec<&T>> {
    GeneratorIterator(|| {
        let mut a = input.iter().map(|x| x).collect::<Vec<_>>();
        let mut p = input.iter().map(|_| 0).collect::<Vec<_>>();

        let mut i = 1;
        while i < input.len() {
            if p[i] < i {
                let j = i % 2 + p[i];
                a.swap(i, j);
                yield a.clone();
                p[i] += 1;
                i = 1;
            } else {
                p[i] = 0;
                i += 1;
            }
        }
    })
}
