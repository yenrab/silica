# Copyright 2026 Lee Scott Barney
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# One suite of a board trial run: trials/.target/<target>/<suite>/Makefile (written by
# trial_target.sh) sets TRIAL_TARGET and BOARD_SUITE and includes this file. The shared integrate
# wrapper (../silica_compiler.mk) writes this directory's log, counts and report; board_suite.sh
# does the work, reading the suite's sources and goldens from trials/<suite>/ without changing
# anything there except the board's own <trial>.<target>.sout / .cur_fail outputs.

_BOARD_TARGETS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
include $(_BOARD_TARGETS_DIR)../silica_compiler.mk

.DEFAULT_GOAL := integrate
.PHONY: integrate-run clean

integrate-run:
	@SILICA_COMPILE_TIMEOUT="$(SILICA_COMPILE_TIMEOUT)" bash "$(_BOARD_TARGETS_DIR)board_suite.sh" "$(TRIAL_TARGET)" "$(BOARD_SUITE)" "$(INTEGRATE_DIR)"

clean:
	@rm -rf "$(INTEGRATE_DIR)src" "$(INTEGRATE_DIR)build"
