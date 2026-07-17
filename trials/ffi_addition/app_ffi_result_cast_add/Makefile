THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
APP_TRIAL_DIR := $(abspath $(THIS_DIR))
APP_LABEL := app_ffi_result_cast_add
APP_EXECUTABLES := dangerous_ffi_result_cast_add
APP_LINK_OBJECTS := dangerous_legacy_stub.o

include $(abspath $(THIS_DIR)/../common_app.mk)

.PHONY: all integrate clean fixtures

all: integrate

integrate: fixtures
integrate:
	$(APP_INTEGRATE_BODY)

clean:
	@cd "$(THIS_DIR)" && rm -f *.sams *.o $(APP_EXECUTABLES) silica.config silica.link .integrate_counts *.sout dangerous_exposure_source
