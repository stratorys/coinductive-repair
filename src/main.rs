fn main() {
    let mut cases = Vec::new();
    for a in 0..16 {
        for b in 0..16 {
            cases.push((a, b));
        }
    }
    cases.extend([
        (u32::MAX, 0),
        (0, u32::MAX),
        (u32::MAX - 1, 1),
        (1, u32::MAX - 1),
    ]);

    println!("{{\"runs\":[");
    for (i, (a, b)) in cases.into_iter().enumerate() {
        let output = coinductive_repair::add(a, b);
        let separator = if i == 0 { "" } else { "," };
        // Every field is numeric, so there is no string to escape.
        println!("{separator}{{\"input\":{{\"a\":{a},\"b\":{b}}},\"output\":{output}}}");
    }
    println!("]}}");
}
