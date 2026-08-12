THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
APP_TRIAL_DIR := $(abspath $(THIS_DIR))
APP_LABEL := app_sidecar_legacy_math_add
APP_EXECUTABLES := dangerous_sidecar_legacy_math_add

include $(abspath $(THIS_DIR)/../common_app.mk)

.PHONY: all integrate clean fixtures

all: integrate

integrate: fixtures
integrate:
	$(APP_INTEGRATE_BODY)

clean:
	@cd "$(THIS_DIR)" && rm -f *.sams *.o *.iface $(APP_EXECUTABLES) silica.config silica.link .integrate_counts silica.compile.order silica.needs_runtime *.sout dangerous_exposure_source
