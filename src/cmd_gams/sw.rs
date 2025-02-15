use clap::*;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

// Create clap subcommand arguments
pub fn make_subcommand() -> Command {
    Command::new("sw")
        .about("Sliding windows around features/peaks")
        .arg(
            Arg::new("target")
                .required(false)
                .num_args(1)
                .index(1)
                .action(ArgAction::Set)
                .value_parser([
                    builder::PossibleValue::new("feature"),
                    builder::PossibleValue::new("peak"),
                ])
                .default_value("feature")
                .help("Which target"),
        )
        .arg(
            Arg::new("action")
                .long("action")
                .short('a')
                .num_args(1)
                .action(ArgAction::Append)
                .value_parser([
                    builder::PossibleValue::new("gc"),
                    builder::PossibleValue::new("gibbs"),
                    builder::PossibleValue::new("count"),
                ])
                .default_value("gc")
                .help("Which statistics"),
        )
        .arg(
            Arg::new("style")
                .long("style")
                .num_args(1)
                .value_parser([
                    builder::PossibleValue::new("center"),
                    builder::PossibleValue::new("intact"),
                ])
                .default_value("intact")
                .help("Style of sliding windows, intact or center"),
        )
        .arg(
            Arg::new("size")
                .long("size")
                .num_args(1)
                .value_parser(value_parser!(i32))
                .default_value("100"),
        )
        .arg(
            Arg::new("max")
                .long("max")
                .num_args(1)
                .value_parser(value_parser!(i32))
                .default_value("20"),
        )
        .arg(
            Arg::new("resize")
                .long("resize")
                .num_args(1)
                .value_parser(value_parser!(i32))
                .default_value("500")
                .help("GC-stat flanking region size"),
        )
        .arg(
            Arg::new("parallel")
                .long("parallel")
                .short('p')
                .value_parser(value_parser!(usize))
                .num_args(1)
                .default_value("1")
                .help("Running in parallel mode, the number of threads"),
        )
        .arg(
            Arg::new("outfile")
                .long("outfile")
                .short('o')
                .num_args(1)
                .default_value("stdout")
                .help("Output filename. [stdout] for screen"),
        )
}

// command implementation
pub fn execute(args: &ArgMatches) -> anyhow::Result<()> {
    //----------------------------
    // Operating
    //----------------------------
    // redis connection
    let mut conn = gams::Conn::new();
    let ctg_of = conn.get_bundle_ctg(None);
    let mut ctgs = vec![];
    for ctg_id in ctg_of.keys().sorted() {
        ctgs.push(ctg_of.get(ctg_id).unwrap().clone())
    }

    eprintln!("{} contigs to be processed", ctgs.len());

    proc_ctg_p(&ctgs, args)?;

    Ok(())
}

fn proc_ctg(ctg: &gams::Ctg, args: &ArgMatches) -> String {
    //----------------------------
    // Args
    //----------------------------
    let opt_size = *args.get_one::<i32>("size").unwrap();
    let opt_max = *args.get_one::<i32>("max").unwrap();
    let opt_resize = *args.get_one::<i32>("resize").unwrap();

    let mut actions: HashSet<String> = HashSet::new();
    for action in args.get_many::<String>("action").unwrap() {
        actions.insert(action.to_string());
    }

    // redis connection
    let mut conn = gams::Conn::new();

    eprintln!("Process {} {}", ctg.id, ctg.range);

    // local caches of GC-content for each ctg
    let mut cache: HashMap<String, f32> = HashMap::new();

    let parent = intspan::IntSpan::from_pair(ctg.chr_start, ctg.chr_end);
    let seq: String = conn.get_seq(&ctg.id);

    // All features in this ctg
    let jsons: Vec<String> = conn.get_scan_values(&format!("feature:{}:*", ctg.id));
    let features: Vec<gams::Feature> = jsons
        .iter()
        .map(|el| serde_json::from_str(el).unwrap())
        .collect();
    eprintln!("\tThere are {} features", features.len());

    let mut out_string = "".to_string();
    for feature in &features {
        let feature_id = &feature.id;
        let feature_range = intspan::Range::from_str(&feature.range);
        let range_start = feature_range.start;
        let range_end = feature_range.end;

        // No need to use Redis counters
        let mut sn: isize = 1;

        let windows = gams::center_sw(&parent, range_start, range_end, opt_size, opt_max);

        for (sw_ints, sw_type, sw_distance) in windows {
            let sw_id = format!("sw:{}:{}", feature_id, sn);

            let mut sw = gams::Sw {
                id: sw_id,
                range: intspan::Range::from(&ctg.chr_id, sw_ints.min(), sw_ints.max()).to_string(),
                sw_type,
                distance: sw_distance,
                gc_content: None,
                gc_mean: None,
                gc_stddev: None,
                gc_cv: None,
                rg_count: None,
            };

            if actions.contains("gc") {
                let gc_content = gams::cache_gc_content(
                    &intspan::Range::from(&ctg.chr_id, sw_ints.min(), sw_ints.max()),
                    &parent,
                    &seq,
                    &mut cache,
                );

                let resized = gams::center_resize(&parent, &sw_ints, opt_resize);
                let re_rg = intspan::Range::from(&ctg.chr_id, resized.min(), resized.max());
                let (gc_mean, gc_stddev, gc_cv) =
                    gams::cache_gc_stat(&re_rg, &parent, &seq, &mut cache, opt_size, opt_size);

                sw.gc_content = Some(gc_content);
                sw.gc_mean = Some(gc_mean);
                sw.gc_stddev = Some(gc_stddev);
                sw.gc_cv = Some(gc_cv);
            }

            sn += 1;

            // outputs
            out_string += &format!("{}\n", sw);
        }
    }

    out_string
}

// Adopt from https://rust-lang-nursery.github.io/rust-cookbook/concurrency/threads.html#create-a-parallel-pipeline
fn proc_ctg_p(ctgs: &Vec<gams::Ctg>, args: &ArgMatches) -> anyhow::Result<()> {
    //----------------------------
    // Args
    //----------------------------
    let mut writer = intspan::writer(args.get_one::<String>("outfile").unwrap());
    let opt_parallel = *args.get_one::<usize>("parallel").unwrap();

    // headers
    let headers = [
        "id",
        "range",
        "type",
        "distance",
        "gc_content",
        "gc_mean",
        "gc_stddev",
        "gc_cv",
        "rg_count",
    ];
    writer.write_all(format!("{}\n", headers.join("\t")).as_ref())?;

    // Channel 1 - Contigs
    let (snd1, rcv1) = crossbeam::channel::bounded::<gams::Ctg>(10);
    // Channel 2 - Results
    let (snd2, rcv2) = crossbeam::channel::bounded::<String>(10);

    crossbeam::scope(|s| {
        //----------------------------
        // Reader thread
        //----------------------------
        s.spawn(|_| {
            for ctg in ctgs {
                snd1.send(ctg.clone()).unwrap();
            }
            // Close the channel - this is necessary to exit the for-loop in the worker
            drop(snd1);
        });

        //----------------------------
        // Worker threads
        //----------------------------
        for _ in 0..opt_parallel {
            // Send to sink, receive from source
            let (sendr, recvr) = (snd2.clone(), rcv1.clone());
            // Spawn workers in separate threads
            s.spawn(move |_| {
                // Receive until channel closes
                for ctg in recvr.iter() {
                    let out_string = proc_ctg(&ctg, args);
                    sendr.send(out_string).unwrap();
                }
            });
        }
        // Close the channel, otherwise sink will never exit the for-loop
        drop(snd2);

        //----------------------------
        // Writer (main) thread
        //----------------------------
        for out_string in rcv2.iter() {
            writer.write_all(out_string.as_ref()).unwrap();
        }
    })
    .unwrap();

    Ok(())
}
