// Included only in the isolated source copy by prepare_threshold_bench.py.
#[derive(Debug, PartialEq)]
enum BenchOutput { Text(String), Tokens(Vec<String>), Tags(Vec<(String, String)>) }

pub fn threshold_benchmark() {
    use std::{hint::black_box, time::Instant, io::Write};
    use std::sync::atomic::Ordering::Relaxed;
    let args: Vec<_> = std::env::args().collect();
    let threads: usize = args[1].parse().unwrap();
    let novel = std::fs::read_to_string(&args[2]).unwrap();
    let mut csv = std::io::BufWriter::new(std::fs::File::create(&args[3]).unwrap());
    let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap();
    let cc = OpenCC::new();
    let sizes = [500,1000,2000,3000,4000,5000,6000,7000,8000,9000,10000,15000,20000,50000,novel.len()];
    let thresholds = [0,1000,2000,3000,4000,5000,6000,7000,8000,9000,usize::MAX];
    let names = ["join_off","join_on","cut_off","cut_on","search_off","search_on","cut_all","tag_off","tag_on","s2t","s2twp","t2s"];
    eprintln!("threads={threads} bytes={} chars={} warmups=3 iterations=12",novel.len(),novel.chars().count());
    writeln!(csv,"threads,target_bytes,excerpt,actual_bytes,chunks,operation,threshold,iteration,ns").unwrap();
    pool.install(|| {
        for size in sizes {
            if args.get(4).map(String::as_str) == Some("excerpts") && size == novel.len() { continue; }
            for excerpt in 0..if size == novel.len() {1} else {3} {
                let mut start = (novel.len()-size) * [11,47,83][excerpt] / 100;
                while !novel.is_char_boundary(start) { start += 1; }
                let mut end = (start+size).min(novel.len());
                // Round upward so a target exactly at a candidate exercises >= dispatch.
                while !novel.is_char_boundary(end) { end += 1; }
                let input = &novel[start..end];
                let chunks = cc.split_string_ranges(input,true).len();
                for (op,name) in names.iter().enumerate() {
                    let run = || {
                        let s = black_box(input);
                        match op {
                            0|1 => BenchOutput::Text(cc.jieba_cut_and_join(s,op==1," ")),
                            2|3 => BenchOutput::Tokens(cc.jieba_cut(s,op==3)),
                            4|5 => BenchOutput::Tokens(cc.jieba_cut_for_search(s,op==5)),
                            6 => BenchOutput::Tokens(cc.jieba_cut_all(s)),
                            7|8 => BenchOutput::Tags(cc.jieba_tag(s,op==8)),
                            9 => BenchOutput::Text(cc.s2t(s,false)),
                            10 => BenchOutput::Text(cc.s2twp(s,false)),
                            _ => BenchOutput::Text(cc.t2s(s,false)),
                        }
                    };
                    BENCH_THRESHOLD.store(usize::MAX,Relaxed);
                    let reference = run();
                    for threshold in thresholds {
                        BENCH_THRESHOLD.store(threshold,Relaxed);
                        assert_eq!(reference,run(),"output mismatch {name} {size} {excerpt} {threshold}");
                        for _ in 0..3 { black_box(run()); }
                    }
                    // Rotate and reverse candidate order between iterations to limit drift.
                    for iteration in 0..12 {
                        for k in 0..thresholds.len() {
                            let index = if iteration%2==0 {(k+iteration)%11} else {(10-k+iteration)%11};
                            let threshold = thresholds[index];
                            BENCH_THRESHOLD.store(threshold,Relaxed);
                            let now = Instant::now();
                            black_box(run()); // includes output destruction, excludes equality/CSV I/O
                            let ns = now.elapsed().as_nanos();
                            writeln!(csv,"{threads},{size},{excerpt},{},{chunks},{name},{threshold},{iteration},{ns}",input.len()).unwrap();
                        }
                    }
                }
            }
            csv.flush().unwrap();
            eprintln!("finished {size} bytes");
        }
    });
}
