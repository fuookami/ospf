use ospf_rust_meta_programming::GeneratorIterator;

pub fn permute<T>(input: &Vec<T>) -> Vec<Vec<&T>> {
    let a = input.iter().map(|x| x).collect::<Vec<_>>();
    let p = input.iter().map(|x| 0).collect::<Vec<_>>();

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

pub fn permute_async<T>(input: &Vec<T>) -> impl Iterator<Item = Vec<&T>> {
    GeneratorIterator(|| {
        let a = input.iter().map(|x| x).collect::<Vec<_>>();
        let p = input.iter().map(|x| 0).collect::<Vec<_>>();

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
