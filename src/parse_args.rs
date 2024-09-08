use getopts;

pub enum Action {
    SeedDatabase { limit: usize },
    RunBot,
    PurgeLinks { dry_run: bool, start_at_id: usize },
    Help(String),
}

pub fn parse_args(args: Vec<String>) -> Action {
    use Action::*;
    let opts = {
        let mut opts = getopts::Options::new();
        opts.opt(
            "",
            "purge",
            "remove dead links from the database",
            "",
            getopts::HasArg::No,
            getopts::Occur::Optional,
        );
        opts.opt(
            "",
            "dry_run", 
            "combine with --purge to print the number of links to be removed instead if running the operation", 
            "", 
            getopts::HasArg::No,
            getopts::Occur::Optional
        );
        opts.opt(
            "",
            "start_at_id", 
            "combine with --purge. Links are deleted in ascending id order, this option sets the first id we check", 
            "0", 
            getopts::HasArg::Yes,
            getopts::Occur::Optional
        );
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
    if matches.opt_present("purge") {
        return PurgeLinks {
            dry_run: matches.opt_present("dry_run"),
            start_at_id: match matches.opt_get_default::<usize>("start_at_id", 0) {
                Ok(id) => id,
                Err(_) => return Help(opts.usage("failed to parse --seed argument to integer")),
            },
        };
    }
    RunBot
}
