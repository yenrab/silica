# Known collisions to resolve when merging the four lanes

Order: frontend, diagnostics, codegen, integrate (smallest tree first).

1. Float parameter shadow. The codegen lane relaxed `emitter_core@use_full_param_shadow` so a body
   with float lets still shadows its float parameters. One of the parked September lanes (a30),
   carried by the integrate lane, patches the same gate for the same trial. Take one of them, not
   both: read what each predicate admits and keep the one that covers float params with float lets.
   The trial keeps the parked lane's name, float_parameter_survives_let_and_call.

2. List-literal parsing. The frontend lane marked list spine pairs to distinguish them from a nested
   list element; the diagnostics lane fixed the closing-bracket consumption in the same function and
   the frontend lane adopted that text verbatim, so those two should merge as identical text.
   The integrate lane may add a tuple-direction fix in the same area.

3. Trial renames. Several trials are renamed off a defect prefix in more than one lane. Where two
   lanes rename the same trial, keep the name that describes the behaviour, not the defect.

4. Duplicate trial directory. The diagnostics lane created
   trials/compile_defects_nested_list_literal_addition; the frontend lane fixed that defect and moved
   the trial to list_addition/nested_list_literal. Drop the compile_defects directory at merge.

5. Untracked build inputs. Every lane copied project_makefiles/platform/{platforms.mk,platforms.patch}
   into its worktree because the committed Makefile includes them and they were untracked in main.
   They must be in the commit, or every future worktree build dies in one second.

6. Linux goldens owed after the merge. Trials renamed or added tonight have Apple Silicon goldens
   only; their linux_aarch64 and linux_x86_64 goldens must be recorded on those machines during the
   fixed-point phase. Known cases: ffi_addition/app_worker_second_message (renamed from
   app_sd18_worker_second_message, whose linux_aarch64 golden went with the old directory), the GNU
   linker variant of dangerous_missing_foreign_symbol_at_link (one-line change: the undefined
   reference moves from `main` to the ffi worker symbol), and every trial the four lanes renamed or
   created. programmer_tools/update_goldens.sh records them from each machine's own run.
