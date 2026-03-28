use opus::app::IdeRuntime;

fn main() {
    let mut runtime = IdeRuntime::seeded_demo();
    println!("{}", runtime.render_overview());

    match runtime.simulate_demo_session() {
        Ok(report) => println!("{report}"),
        Err(err) => {
            eprintln!("demo failed: {err}");
            std::process::exit(1);
        }
    }
}
