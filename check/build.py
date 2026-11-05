import re, pathlib, subprocess, sys
here = pathlib.Path(__file__).parent
b = here.parent / 'mixins' / 'cinematic_interactions'
t = (here / 'template.rs').read_text().replace('//SCENE//', (b / 'scene.rs').read_text())
for f in sorted(b.glob('*.rs')):
    if f.name in ('scene.rs', 'dx12_terrain.rs'):
        continue
    name = f.stem
    src = re.sub(r'\bfn cinematic_\w+\(', f'fn __{name}(', f.read_text(), count=1)
    key = f'//{name.upper()}//'
    if key not in t:
        sys.exit(f'no place for {f.name}')
    t = t.replace(key, src)
(here / 'check.rs').write_text(t)
winit = here / 'winit_stub.rs'
subprocess.run(['rustc', '--edition', '2021', '--crate-type', 'lib', '--crate-name', 'winit', str(winit), '-o', '/tmp/libwinit.rlib'], check=True)
args = ['--edition', '2024', '--crate-name', 'horizon_client', '-L', '/tmp', '--extern', 'glam=/tmp/libglam.rlib', '--extern', 'winit=/tmp/libwinit.rlib']
r = subprocess.run(['rustc', *args, '--crate-type', 'lib', 'check.rs', '-o', '/tmp/ci_check.rlib'], cwd=here, capture_output=True, text=True)
print(r.stderr[-12000:])
if r.returncode or '--test' not in sys.argv:
    sys.exit(r.returncode)
(here / 'check_test.rs').write_text(t + '\n#[cfg(test)]\n#[path = "' + str((here.parent / 'test' / 'tests.rs').resolve()) + '"]\nmod tests;\n')
r = subprocess.run(['rustc', *args, '--test', 'check_test.rs', '-o', '/tmp/ci_tests'], cwd=here, capture_output=True, text=True)
print(r.stderr[-12000:])
if r.returncode:
    sys.exit(r.returncode)
sys.exit(subprocess.run(['/tmp/ci_tests', '--test-threads=1']).returncode)
