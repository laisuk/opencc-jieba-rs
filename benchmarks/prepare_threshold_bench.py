"""Create an isolated, instrumented source copy; production sources stay untouched."""
from pathlib import Path
import shutil

root = Path(__file__).resolve().parents[1]
dest = root / 'target' / 'threshold-bench'
dest.mkdir(parents=True, exist_ok=True)
shutil.copytree(root / 'src', dest / 'src', dirs_exist_ok=True)
for name in ['Cargo.toml', 'Cargo.lock', 'build.rs']:
    shutil.copy2(root / name, dest / name)
manifest = (dest / 'Cargo.toml').read_text()
start = manifest.index('[workspace]')
end = manifest.index('[package]')
manifest = '[workspace]\nmembers = []\n\n' + manifest[end:]
manifest = manifest[:manifest.index('[[example]]')]
(dest / 'Cargo.toml').write_text(manifest)
p = dest / 'src/opencc.rs'
s = p.read_text(encoding='utf-8')
assert s.count('input_len >= PARALLEL_THRESHOLD') == 1
s = s.replace('input_len >= PARALLEL_THRESHOLD', 'input_len >= BENCH_THRESHOLD.load(std::sync::atomic::Ordering::Relaxed)')
s += '\nstatic BENCH_THRESHOLD: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(PARALLEL_THRESHOLD);\n'
s += '\ninclude!("threshold_harness.rs");\n'
p.write_text(s, encoding='utf-8')
shutil.copy2(root / 'benchmarks/threshold_harness.rs', dest / 'src/threshold_harness.rs')
with (dest / 'src/lib.rs').open('a') as f:
    f.write('\npub use opencc::threshold_benchmark;\n')
(dest / 'src/bin').mkdir(exist_ok=True)
(dest / 'src/bin/threshold_benchmark.rs').write_text('fn main() { opencc_jieba_rs::threshold_benchmark(); }\n')
print(dest)
