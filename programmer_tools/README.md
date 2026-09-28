# programmer_tools

Scripts for building Silica across machines, running its trials, and keeping the recorded
expectations honest. Every script prints its own manual with `-h` or `--help`, and the ones that do
anything expensive accept `--list` to show what a run would do without doing it.

## Working on the compiler?

Read **[A change, end to end](https://yenrab.github.io/silica/build-and-test/#a-change-end-to-end)**
first. It gives the order to work in, from making your modification to reaching a new fixed point,
with a table of which steps a given kind of change needs. The scripts below are what it calls.

## Which script do I want?

| I want to | Use |
| --- | --- |
| Build the compiler everywhere it is supported | `build_all_platforms.sh` |
| Run the trials on every platform | `run_trials_all_platforms.sh` |
| Do both, in one command | `build_and_trial_all_platforms.sh` |
| Find out what is really broken after changing the compiler | `rebuild_refresh_verify.sh` |
| Re-record assembly goldens after an intended change | `update_goldens.sh` |

## The machine file

Three of these scripts read `.silica_build_hosts` in the repository root: one line per platform,
`<emit target> <user@host|local> <repository path>`. It is written the first time a script asks, and
reused silently afterwards. **It is gitignored and must never be committed**, because it names your
machines.

Every remote connection is public/private key ssh. The scripts run `ssh` in batch mode and never type
a password, so set a key up first with `ssh-keygen` and `ssh-copy-id user@host`, and check it with
`ssh -o BatchMode=yes user@host true`.

## What each script does

### build_all_platforms.sh
Builds Silica for every supported platform, from whatever machine you start on.

An OS-hosted platform compiles its own code, so it builds natively on its own machine and reaches its
own fixed point there: this script does that here, and over ssh on the machines you name. It does not
cross-build a hosted platform, with one exception, a machine that has no compiler yet, which it
offers to hand a first one to by building here, emitting the assembly here and linking there. A raw
platform such as `ESP32-S3_raw` has no operating system and cannot host a compiler, so its cross
compiler is built here.

Per machine the work is build, gen1, gen2, fixed-point check, publish; `--no-fixpoint` stops after the
build. Platform selection: nothing selects the remembered set, `--targets "<list>"` selects for one
run, `--set_targets "<list>"` asks and rewrites the file, `--refresh_targets` re-asks, `--remote
<target>=<user@host>[:<path>]` names one machine inline, `--local-only` stays on this machine.

### run_trials_all_platforms.sh
Runs the full trial tree on each platform with that platform's own compiler. It never builds
anything: each machine must already have a compiler at `binaries/silica-compiler`, and a raw target is
driven from this machine with the cross compiler built for it.

Remote machines run at the same time as each other; this machine and any board run one after another,
because both take the tree-wide lock at `trials/.integrate.lock`. `--sync` copies trials, the standard
library and the project makefiles to each remote machine first. `--board` includes a raw target, which
is off by default because the board has to be plugged in. Short names `mac`, `pi`, `nix`, `esp32`,
`all` and `hosted` are accepted anywhere a platform is.

### build_and_trial_all_platforms.sh
The two above in one command: build every indicated platform, then run the full trial tree on each.
Both halves read the same machine file, so platforms named once are used twice. The trials do not run
if a build fails, because a trial result only means something when it came from a compiler that built
cleanly. `--skip-build` and `--skip-trials` run half of it.

### rebuild_refresh_verify.sh
Answers the question "the compiler changed, what is actually broken now?".

Per hosted platform, in order: clean and build; run the whole tree once, which is what produces the
assembly the goldens come from; record the goldens with `update_goldens.sh --apply`, which refreshes
only trials whose program output already matched; run the whole tree again. **The second run is the
answer**: every stale-expectation failure is gone, so whatever is still red is a behaviour difference.

A raw target is built here as a cross compiler and its trials run on the attached board, with no
golden step, because the board harness compares program output only and never assembly. Raw targets
run last, since the board takes this machine's trial lock.

With no `--targets` it asks which platforms to run and, for each hosted machine it does not know, for
the connection; if the machine file already exists it shows what it remembers and lets you keep or
change it, with every answer prefilled. It ends with a per-platform table of PASS, FAIL, BUILD FAILED,
NO MACHINE or UNREACHABLE, and exits non-zero unless every platform passed.

### update_goldens.sh
Re-records `.ascomp` assembly goldens from the `.sams` a trial run left behind.

**It is a dry run by default and needs `--apply` to write anything.** Goldens in this project are
hand-derived and are not auto-regenerated: a golden that changes is a behaviour change in the
compiler, and blessing one in bulk can bless a defect as easily as a fix. The dry run prints every
golden that would change with a per-file added/removed line count, so a three-line drift and a
three-thousand-line rewrite do not look alike.

It never touches `.scout` or `.golden_fail` files, skips `__silica_runtime`, which the harness never
compares, and refuses a `.sams` whose instruction set does not match the platform being blessed, which
is what catches a stale cross-build. `--suite <dir>` or bare paths limit the scope; `--create-missing`
is needed before it will record a golden that does not exist yet.

## Subdirectories

- **`defect_batch/`** — artefacts of the September 2026 defect-clearing batch: `merge_lanes.sh` merges
  the lane branches onto main in a safe order, dry by default; `merge_notes.md` lists the known
  collisions; `defect_ledger.md` is the full record of every defect, what was proven and what was
  retracted; `post_merge_harness_queue.md` holds harness work waiting for a quiet tree, including
  `drop_rust_lld.py`, a tested transform that removes the retired Rust linker from the trial makefiles.
- **`train_trial_generator/`** — generates paired Silica trials for model training; see its own README.

## The jsonld files

`silica-builder.jsonld`, `silica-compaid.jsonld` and `silica-error-code-scheme.jsonld` are tool
definitions rather than scripts, and are not run from here.
