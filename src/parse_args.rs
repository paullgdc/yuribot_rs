use getopts;

pub enum Action {
    SeedDatabase { limit: usize },
    RunBot,
    Help(String),
}

pub fn parse_args(args: Vec<String>) -> Action {
    use Action::*;
    let opts = {
        let mut opts = getopts::Options::new();
        opts.opt(
            "s",
            "seed",
            "initializes the database with pics from /top, defaults to 200 if no number is supplied",
            "N",
            getopts::HasArg::Maybe,
            getopts::Occur::Optional,
        );
        opts.optflag("h", "help", "prints the help");
        opts
    };
    let matches = match opts.parse(args) {
        Ok(m) => m,
        Err(_) => return Help(opts.usage("Failed while parsing args")),
    };
    if matches.opt_present("help") {
        return Help(opts.usage(""));
    }
    if matches.opt_present("seed") {
        return match matches.opt_get_default::<usize>("seed", 200_usize) {
            Ok(i) => SeedDatabase { limit: i },
            Err(_) => Help(opts.usage("failed to parse --seed argument to integer")),
        };
    }
    RunBot
}
