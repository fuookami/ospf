mod core;
mod framework;

fn run_core(id: usize) -> Result<(), Box<dyn std::error::Error>> {
    match id {
        1 => core::run_demo1(),
        2 => core::run_demo2(),
        3 => core::run_demo3(),
        4 => core::run_demo4(),
        5 => core::run_demo5(),
        6 => core::run_demo6(),
        7 => core::run_demo7(),
        8 => core::run_demo8(),
        9 => core::run_demo9(),
        10 => core::run_demo10(),
        11 => core::run_demo11(),
        12 => core::run_demo12(),
        13 => core::run_demo13(),
        14 => core::run_demo14(),
        15 => core::run_demo15(),
        16 => core::run_demo16(),
        17 => core::run_demo17(),
        _ => Err(format!("unknown core demo id: {}", id).into()),
    }
}

fn run_all_core() -> Result<(), Box<dyn std::error::Error>> {
    for id in 1..=17 {
        println!("\n----- core demo {} -----", id);
        run_core(id)?;
    }
    Ok(())
}

fn print_usage() {
    println!("usage:");
    println!("  ospf-rust-example core:1");
    println!("  ospf-rust-example core:all");
    println!("  ospf-rust-example core:shortcuts");
    println!("  ospf-rust-example core:capability-gate");
    println!("  ospf-rust-example framework:demo1");
    println!("  ospf-rust-example framework:demo2");
    println!("  ospf-rust-example framework:demo3");
    println!("  ospf-rust-example framework:demo4");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(String::as_str).unwrap_or("core:1");

    let result = match command {
        "core:all" => run_all_core(),
        "core:shortcuts" => core::run_core_shortcuts(),
        "core:capability-gate" => core::run_capability_gate(),
        "framework:demo1" => framework::run_demo1(),
        "framework:demo2" => framework::run_demo2(),
        "framework:demo3" => framework::run_demo3(),
        "framework:demo4" => framework::run_demo4(),
        _ if command.starts_with("core:") => {
            let id = command.trim_start_matches("core:").parse::<usize>();
            match id {
                Ok(id) => run_core(id),
                Err(_) => Err(format!("invalid core demo command: {}", command).into()),
            }
        }
        _ => {
            print_usage();
            Err(format!("unknown command: {}", command).into())
        }
    };

    if let Err(err) = result {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}
