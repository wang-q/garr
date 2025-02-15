use clap::*;
use redis::{Commands, RedisResult};

use rand::Rng;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::thread::sleep;

// Create clap subcommand arguments
pub fn make_subcommand() -> Command {
    Command::new("status")
        .about("Test Redis config and connection")
        .after_help(
            r#"
List of actions:

* cli:  find `redis-cli` in $PATH
* test: redis.rs functionality
* info: Command INFO     - memory usage of the database
* drop: Command FLUSHDB  - drop the database for accepting new data
* dump: Command SAVE     - export of the contents of the database
* stop: Command SHUTDOWN - quit the server

"#,
        )
        .arg(
            Arg::new("action")
                .index(1)
                .num_args(1)
                .default_value("info")
                .help("What to do"),
        )
        .arg(
            Arg::new("file")
                .index(2)
                .num_args(1)
                .default_value("backup.rdb")
                .help("Target filename"),
        )
}

// command implementation
pub fn execute(args: &ArgMatches) -> anyhow::Result<()> {
    let file = args.get_one::<String>("file").unwrap();
    match args.get_one::<String>("action").unwrap().as_str() {
        "cli" => {
            cli();
        }
        "test" => {
            basics();
            hash();
            list();
            set();
            sorted_set();
            pipe_atomic();
            script();
        }
        "info" => {
            info();
        }
        "drop" => {
            gams::db_drop();
        }
        "dump" => {
            dump(file)?;
        }
        "stop" => {
            stop();
        }
        // TODO: restore
        // TODO: server
        _ => unreachable!(),
    };

    Ok(())
}

fn info() {
    let mut conn = gams::connect();
    let info: redis::InfoDict = redis::cmd("INFO")
        .query(&mut conn)
        .expect("Failed to execute INFO");

    let mut output: BTreeMap<String, String> = BTreeMap::new();
    for key in [
        "redis_version",
        "os",
        "used_memory_human",
        "total_system_memory_human",
        "maxmemory_human",
        "total_connections_received",
        "total_commands_processed",
    ] {
        output.insert(key.to_string(), info.get(key).unwrap());
    }

    let config: Vec<String> = redis::cmd("CONFIG")
        .arg("GET")
        .arg("dir")
        .query(&mut conn)
        .expect("Failed to execute CONFIG");
    config.windows(2).for_each(|w| {
        output.insert(w[0].to_string(), w[1].to_string());
    });

    eprintln!("output = {:#?}", output);
}

fn cli() {
    let res_redis = std::process::Command::new("redis-cli")
        .arg("--version")
        .output();

    if let Ok(output) = res_redis {
        let msg = std::str::from_utf8(output.stdout.as_ref()).unwrap().trim();
        eprintln!("Find `{:#?}` in $PATH", msg);
    } else {
        eprintln!("`redis-cli` not found in $PATH");
        let res_memurai = std::process::Command::new("memurai-cli")
            .arg("--version")
            .output();
        if let Ok(output) = res_memurai {
            let msg = std::str::from_utf8(output.stdout.as_ref()).unwrap().trim();
            eprintln!("Find `{:#?}` in $PATH", msg);
        }
    }
}

fn dump(file: &str) -> anyhow::Result<()> {
    let mut conn = gams::connect();

    // When LASTSAVE changed, the saving is completed
    let start: i32 = redis::cmd("LASTSAVE")
        .query(&mut conn)
        .expect("Failed to execute LASTSAVE");

    let output: String = redis::cmd("BGSAVE")
        .query(&mut conn)
        .expect("Failed to execute SAVE");
    println!("{}", output);

    loop {
        let cur: i32 = redis::cmd("LASTSAVE")
            .query(&mut conn)
            .expect("Failed to execute LASTSAVE");

        eprintln!("Sleep 1 sec");
        sleep(std::time::Duration::from_secs(1));

        if cur != start {
            eprintln!("Redis BGSAVE completed");
            break;
        }
    }

    // try backup
    let config: Vec<String> = redis::cmd("CONFIG")
        .arg("GET")
        .arg("dir")
        .query(&mut conn)
        .expect("Failed to execute CONFIG");

    let redis_dir = config.get(1).unwrap();

    let mut rdb = Path::new(redis_dir).to_path_buf();
    rdb.push("dump.rdb");

    if !rdb.is_file() {
        // redis-server runs inside WSL
        rdb = Path::new("dump.rdb").to_path_buf();
        if !rdb.is_file() {
            eprintln!("Can't find dump.rdb");
            return Ok(());
        }
    }

    eprintln!("rdb = {:#?}, dest = {:#?}", rdb, file);
    fs::copy(rdb, Path::new(file))?;

    Ok(())
}

fn stop() {
    let mut conn = gams::connect();

    redis::cmd("SHUTDOWN")
        .arg("SAVE")
        .query::<()>(&mut conn)
        .unwrap_err();
    eprintln!("Executed SHUTDOWN SAVE");
}

fn basics() {
    let mut conn = gams::connect();
    println!("******* Running SET, GET, INCR commands *******");

    let _: () = redis::cmd("SET")
        .arg("foo")
        .arg("bar")
        .query(&mut conn)
        .expect("Failed to execute SET for 'foo'");

    let bar: String = redis::cmd("GET")
        .arg("foo")
        .query(&mut conn)
        .expect("Failed to execute GET for 'foo'");
    println!("value for 'foo' = {}", bar);

    //INCR and GET using high-level commands
    let _: () = conn
        .incr("counter", 2)
        .expect("Failed to execute INCR for 'counter'");

    let val: i32 = conn
        .get("counter")
        .expect("Failed to execute GET for 'counter'");

    println!("counter = {}", val);
}

fn hash() {
    let mut conn = gams::connect();

    println!("******* Running HASH commands *******");

    let mut driver: BTreeMap<String, String> = BTreeMap::new();
    let prefix = "redis-driver";

    driver.insert(String::from("name"), String::from("redis-rs"));
    driver.insert(String::from("version"), String::from("0.20.0"));
    driver.insert(
        String::from("repo"),
        String::from("https://github.com/mitsuhiko/redis-rs"),
    );

    let _: () = redis::cmd("HSET")
        .arg(format!("{}:{}", prefix, "rust"))
        .arg(driver)
        .query(&mut conn)
        .expect("Failed to execute HSET");

    let info: BTreeMap<String, String> = redis::cmd("HGETALL")
        .arg(format!("{}:{}", prefix, "rust"))
        .query(&mut conn)
        .expect("Failed to execute HGETALL");
    println!("info for rust redis driver: {:?}", info);

    let _: () = conn
        .hset_multiple(
            format!("{}:{}", prefix, "go"),
            &[
                ("name", "go-redis"),
                ("version", "8.4.6"),
                ("repo", "https://github.com/go-redis/redis"),
            ],
        )
        .expect("Failed to execute HSET");

    let repo_name: String = conn
        .hget(format!("{}:{}", prefix, "go"), "repo")
        .expect("Failed to execute HGET");
    println!("go redis driver repo name: {:?}", repo_name);

    let (go_name, go_repo): (String, String) = conn
        .hget(format!("{}:{}", prefix, "go"), &["name", "repo"])
        .expect("Failed to execute HGET");
    println!("go redis driver: {:?} {:?}", go_name, go_repo);
}

fn list() {
    let mut conn = gams::connect();
    println!("******* Running LIST commands *******");

    let list_name = "items";

    let _: () = redis::cmd("LPUSH")
        .arg(list_name)
        .arg("item-1")
        .query(&mut conn)
        .expect("Failed to execute LPUSH for 'items'");

    let item: String = conn
        .lpop(list_name, None)
        .expect("Failed to execute LPOP for 'items'");
    println!("first item: {}", item);

    let _: () = conn.rpush(list_name, "item-2").expect("RPUSH failed");
    let _: () = conn.rpush(list_name, "item-3").expect("RPUSH failed");

    let len: isize = conn
        .llen(list_name)
        .expect("Failed to execute LLEN for 'items'");
    println!("no. of items in list = {}", len);

    let items: Vec<String> = conn
        .lrange(list_name, 0, len - 1)
        .expect("Failed to execute LRANGE for 'items'");
    println!("listing items in list");

    for item in items {
        println!("item: {}", item)
    }
}

fn set() {
    let mut conn = gams::connect();
    println!("******* Running SET commands *******");

    let set_name = "users";

    let _: () = conn
        .sadd(set_name, "user1")
        .expect("Failed to execute SADD for 'users'");
    let _: () = conn
        .sadd(set_name, "user2")
        .expect("Failed to execute SADD for 'users'");

    let ismember: bool = redis::cmd("SISMEMBER")
        .arg(set_name)
        .arg("user1")
        .query(&mut conn)
        .expect("Failed to execute SISMEMBER for 'users'");
    println!("does user1 exist in the set? {}", ismember); //true

    let users: Vec<String> = conn.smembers(set_name).expect("Failed to execute SMEMBERS");
    println!("listing users in set"); //true

    for user in users {
        println!("user: {}", user)
    }
}

fn sorted_set() {
    let mut conn = gams::connect();
    println!("******* Running SORTED SET commands *******");

    let sorted_set = "leaderboard";

    let _: () = redis::cmd("ZADD")
        .arg(sorted_set)
        .arg(rand::thread_rng().gen_range(1..10))
        .arg("player-1")
        .query(&mut conn)
        .expect("Failed to execute ZADD for 'leaderboard'");

    //add many players
    for num in 2..=5 {
        let _: () = conn
            .zadd(
                sorted_set,
                String::from("player-") + &num.to_string(),
                rand::thread_rng().gen_range(1..10),
            )
            .expect("Failed to execute ZADD for 'leaderboard'");
    }

    let count: isize = conn
        .zcard(sorted_set)
        .expect("Failed to execute ZCARD for 'leaderboard'");

    let leaderboard: Vec<(String, isize)> = conn
        .zrange_withscores(sorted_set, 0, count - 1)
        .expect("ZRANGE failed");
    println!("listing players and scores");

    for item in leaderboard {
        println!("{} = {}", item.0, item.1)
    }
}

fn pipe_atomic() {
    let mut conn = gams::connect();
    println!("******* Running MULTI EXEC commands *******");

    redis::pipe()
        .cmd("ZADD")
        .arg("ctg-s:I")
        .arg(1)
        .arg("ctg:I:1")
        .ignore()
        .cmd("ZADD")
        .arg("ctg-s:I")
        .arg(100001)
        .arg("ctg:I:2")
        .ignore()
        .cmd("ZADD")
        .arg("ctg-e:I")
        .arg(100000)
        .arg("ctg:I:1")
        .ignore()
        .cmd("ZADD")
        .arg("ctg-e:I")
        .arg(230218)
        .arg("ctg:I:2")
        .ignore()
        .execute(&mut conn);

    let res_s: BTreeSet<String> = conn.zrangebyscore("ctg-s:I", 0, 1000).unwrap();
    eprintln!("res = {:#?}", res_s);

    let res_e: BTreeSet<String> = conn.zrangebyscore("ctg-e:I", 1100, "+inf").unwrap();
    eprintln!("res = {:#?}", res_e);

    let res: Vec<_> = res_s.intersection(&res_e).collect();
    eprintln!("res = {:#?}", res);

    // MULTI
    // ZRANGESTORE tmp-s:I ctg-s:I 0 1000 BYSCORE
    // ZRANGESTORE tmp-e:I ctg-e:I 1100 +inf BYSCORE
    // ZINTERSTORE tmp-ctg:I 2 tmp-s:I tmp-e:I AGGREGATE MIN
    // DEL tmp-s:I tmp-e:I
    // ZPOPMIN tmp-ctg:I
    // EXEC
}

fn script() {
    let mut conn = gams::connect();
    println!("******* Running Lua Scripts *******");

    let script = redis::Script::new(
        r###"
return tonumber(ARGV[1]) + tonumber(ARGV[2]);
"###,
    );
    let res: RedisResult<i32> = script.arg(1).arg(2).invoke(&mut conn);
    eprintln!("res = {:#?}", res);

    // https://github.com/redis/redis/issues/7#issuecomment-596464166
    // https://stackoverflow.com/questions/52167955/use-lua-script-with-scan-command-to-obtain-the-list
    let script = redis::Script::new(
        r###"
local cursor = 0;
local count = 0;
repeat
    local result = redis.call('SCAN', cursor, 'MATCH', ARGV[1], 'COUNT', ARGV[2])
    cursor = result[1];
    local count_delta = #result[2];
    count = count + count_delta;
until cursor == "0";
return count;
"###,
    );
    let res: RedisResult<i32> = script.arg("ctg*").arg(1000).invoke(&mut conn);
    eprintln!("res = {:#?}", res);

    let script = redis::Script::new(
        r###"
local cursor = 0;
local list = {};
repeat
    local result = redis.call('SCAN', cursor, 'MATCH', ARGV[1], 'COUNT', ARGV[2])
    cursor = result[1];
    for _, k in ipairs(result[2]) do
        list[#list+1] = k
    end
until cursor == "0";
return list;
"###,
    );
    let res: RedisResult<Vec<String>> = script.arg("ctg*").arg(1000).invoke(&mut conn);
    eprintln!("res = {:#?}", res);

    let script = redis::Script::new(
        r###"
local cursor = 0;
local list = {};
repeat
    local result = redis.call('SCAN', cursor, 'MATCH', ARGV[1], 'COUNT', ARGV[2])
    cursor = result[1];
    for _, k in ipairs(result[2]) do
        list[#list+1] = redis.call('GET', k)
    end
until cursor == "0";
return list;
"###,
    );
    let res: RedisResult<Vec<String>> = script.arg("foo*").arg(1000).invoke(&mut conn);
    eprintln!("res = {:#?}", res);
}
