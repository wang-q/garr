# Change Log

## Unreleased - ReleaseDate

* Rewrite some parts in `libs/redis.rs`
    * Make it OOP
    * Minimize the use of redis commands in each subcommand
    * Implement pipelines. Up to 10x times faster
    * Use server-side lua scripting for `SCAN MATCH`
* Add structs `Ctg`, `Feature`, `Rg`, and `Peak` in `libs/data.rs`
    * Store serialized json to Redis
* Serializing to .tsv via serde
* By default, `gams wave` will write merged peaks instead of the sliding windows with signals
    * --signal turn off this behavior

* Rename `gams range` to `gams rg`
* Rename `gams sliding` to `gams wave`
* Rename `gams fsw` to `gams sw`
* Add --action to `gams sw`
* Add --parallel to `gams wave` and `gams sw`
* Add --seq to `gams locate`
* Add --count to `gams locate`
* Using lua scripts in `gams clear`
* Support sql in `gams-stat`
* Enhance `gams status dump`

* Bump deps
    * `clap` v4
    * `polars` v0.41

* Add more benchmarks
* Update docs

## 0.3.1 - 2022-06-15

* Add `gams anno`
* Add `--range` to `gams tsv`

* Switch to `clap` v3.2
* Switch to `intspan` v0.7.1
* Remove TLS
* Remove SNR from gc_stat()

* Rename .ranges to .rg

* Add more tests
* Add more benchmarks

## 0.3.0 - 2022-05-13

* Rename `gams range` to `gams feature`
* Rename `gams rsw` to `gams fsw`
* Rename `gams pos` to `gams range`
* Rename `gams wave` to `gams peak`
* Add `gams locate`
* Add `gams clear`

* Avoid get_scan_vec() inside loops
    * Speedup `gams feature`
    * Speedup `gams fsw`
    * Speedup `gams range`
    * Speedup `gams peak`

* Rename .pos.txt to .ranges
* Rename chr_name to chr_id

* `redis.rs`
    * Rename find_one() to find_one_z()
    * Add find_one_l()
    * Add build_idx_ctg()
    * Add get_idx_ctg()
    * Add find_one_idx()
    * Add get_vec_chr()
    * Add get_vec_ctg()
    * Add get_vec_feature()
    * Add get_vec_peak()

* Add more tests

## 0.2.1 - 2022-05-10

* Separate gams-stat
* Restructuring of documents
* Store gzipped seq to redis
* cache_gc_content and cache_gc_stat()

## 0.2.0 - 2022-04-23

* Renamed to `gams`

* Add `gams status stop`

* Queries done by clickhouse
    * `gams env` creates sqls
    * Stats of ctg and rsw

* `gams stat`
    * `Polars` can't read sqls, so use the built-in queries
    * `Datafusion` makes the compilation extremely slow

## 0.1.0 - 2021-08-27

* Skeletons, need to be filled

* Subcommands
    * env
    * gen
    * pos
    * range
    * rsw
    * sliding
    * status
    * tsv
    * wave
