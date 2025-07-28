# lwc

`lwc` is a parallelized, directory-aware alternative to `wc`. It counts lines,
words, characters, bytes, and filesystem elements, with content stats limited to
valid UTF-8 files. That’s no problem for most use cases, but things might get a
little freaky with binary files or text in certain Asian encodings.

![lwc example](https://github.com/psczlek/lwc/blob/main/img/lwc.png)

## Building

Clone the repo `cd` into it, and run either:

```
cargo b     # For debug build
```

or

```
cargo b -r  # For release build
```

For instructions on installing the Rust toolchain refer to the official Rust website.

## Examples

Count lines, words, characters, and bytes in each input files:

```
$ lwc coreutils/src/wc.c coreutils/src/cat.c
980 lines 3548 words 29771 chars 29771 bytes ==> coreutils/src/wc.c
822 lines 2950 words 24484 chars 24485 bytes ==> coreutils/src/cat.c

1802 lines 6498 words 54255 chars 54256 bytes
```

Use `lwc` with piped input from stdin:

```
$ cat coreutils/src/ls.c | lwc
5631 lines 20392 words 169267 chars 169267 bytes
```

Recursively process all files in a directory and print per-file stats:

```
$ lwc -r linux
#
# ...
#
485 lines 1318 words 10045 chars 10046 bytes ==> linux/block/opal_proto.h
3350 lines 8960 words 79845 chars 79846 bytes ==> linux/block/sed-opal.c
950 lines 2736 words 24141 chars 24141 bytes ==> linux/block/fops.c
208 lines 555 words 5655 chars 5655 bytes ==> linux/block/blk-crypto-sysfs.c
964 lines 2421 words 26498 chars 26498 bytes ==> linux/block/blk-sysfs.c
95 lines 234 words 2509 chars 2509 bytes ==> linux/block/blk-mq-debugfs.h
1103 lines 3308 words 30575 chars 30575 bytes ==> linux/block/mq-deadline.c
314 lines 965 words 8501 chars 8501 bytes ==> linux/block/blk-ia-ranges.c
538 lines 2049 words 15985 chars 15985 bytes ==> linux/block/blk-flush.c
1366 lines 4013 words 35516 chars 35516 bytes ==> linux/block/bdev.c
862 lines 2303 words 20250 chars 20250 bytes ==> linux/block/elevator.c
614 lines 2084 words 17400 chars 17400 bytes ==> linux/block/blk-crypto.c
316 lines 1224 words 8644 chars 8644 bytes ==> linux/block/early-lookup.c
182 lines 541 words 5136 chars 5136 bytes ==> linux/block/blk-rq-qos.h
92 lines 270 words 2172 chars 2172 bytes ==> linux/block/blk-mq-cpumap.c
2268 lines 7414 words 61031 chars 61033 bytes ==> linux/block/blk-cgroup.c
277 lines 667 words 6428 chars 6428 bytes ==> linux/block/bsg.c
7728 lines 38162 words 271944 chars 271944 bytes ==> linux/block/bfq-iosched.c

40891187 lines 126624052 words 1519373640 chars 1520833273 bytes
```

Count directory elements (subdirs, fifos, sockets, etc.) instead of file contents:

```
$ lwc -d linux
25 subdirs 17 files ==> linux
```

Combine recursive processing and directory element counting for a nested directory
structure:

```
$ lwc -dr linux
#
# ...
#
29 files ==> linux/kernel/irq
33 files ==> linux/kernel/locking
44 files ==> linux/kernel/sched
16 files ==> linux/kernel/cgroup
42 files ==> linux/kernel/time
21 files ==> linux/kernel/rcu
9 files ==> linux/kernel/configs
15 files ==> linux/kernel/dma
8 files ==> linux/kernel/events
12 files ==> linux/kernel/printk
1 subdir 64 files ==> linux/kernel/bpf
1 subdir 5 files ==> linux/kernel/bpf/preload
6 files ==> linux/kernel/bpf/preload/iterators
1 subdir 4 files ==> linux/kernel/debug
11 files ==> linux/kernel/debug/kdb
1 subdir 75 files ==> linux/block
25 files ==> linux/block/partitions

6089 subdirs 89859 files 80 symlinks
```

Display only the final total for all recursively processed files in a directory:

```
$ lwc -rt coreutils
200832 lines 899771 words 6469163 chars 6469348 bytes
```

Show a total count of subdirectories, files, etc., suppressing individual listings:

```
$ lwc -drt coreutils
93 subdirs 1247 files
```

```
$ time lwc -rt linux coreutils
41092019 lines 127523823 words 1525842803 chars 1527302621 bytes
real 2.74
user 4.24
sys 2.92
```
