.SUFFIXES:

TARGET := ds-controller
BUILD := build
ARM7_TARGET := ds-controller-arm7
TOPDIR ?= $(CURDIR)
ARM7_ELF ?= $(TOPDIR)/$(BUILD)/arm7/$(ARM7_TARGET).elf

ifneq ($(filter test clean,$(MAKECMDGOALS)),)

.PHONY: test clean

test:
	@$(MAKE) --no-print-directory -C tests test

clean:
	@echo clean ...
	@rm -fr $(BUILD)

else

.DEFAULT_GOAL := all

ifeq ($(strip $(DEVKITARM)),)
$(error "Please set DEVKITARM in your environment. export DEVKITARM=<path to>devkitARM")
endif

include $(DEVKITARM)/ds_rules

_ARM7_ELF := -7 $(ARM7_ELF)

SOURCES := source
INCLUDES := source common
DATA :=
GRAPHICS :=
NITRO :=
PC_PORT ?= 26760
PC_IP ?= 192.0.2.1

ARCH := -march=armv5te -mtune=arm946e-s

CFLAGS := -std=gnu11 -g -Wall -O2 -ffunction-sections -fdata-sections $(ARCH) $(INCLUDE) -DARM9 \
          -DDS_CONTROLLER_PC_IP=\"$(PC_IP)\" -DDS_CONTROLLER_PC_PORT=$(PC_PORT)
CXXFLAGS := $(CFLAGS) -fno-rtti -fno-exceptions
ASFLAGS := -g $(ARCH)
LDFLAGS = -specs=ds_arm9.specs -g $(ARCH) -Wl,-Map,$(notdir $*.map)

LIBS := -ldswifi9 -lfat -lnds9
LIBDIRS := $(LIBNDS) $(PORTLIBS)

ifneq ($(BUILD),$(notdir $(CURDIR)))

export OUTPUT := $(CURDIR)/$(BUILD)/$(TARGET)
export TOPDIR := $(CURDIR)
export ARM7_ELF := $(CURDIR)/$(BUILD)/arm7/$(ARM7_TARGET).elf
export VPATH := $(foreach dir,$(SOURCES),$(CURDIR)/$(dir))
export DEPSDIR := $(CURDIR)/$(BUILD)

CFILES := $(foreach dir,$(SOURCES),$(notdir $(wildcard $(dir)/*.c)))
CPPFILES := $(foreach dir,$(SOURCES),$(notdir $(wildcard $(dir)/*.cpp)))
SFILES := $(foreach dir,$(SOURCES),$(notdir $(wildcard $(dir)/*.s)))

ifeq ($(strip $(CPPFILES)),)
export LD := $(CC)
else
export LD := $(CXX)
endif

export OFILES_SOURCES := $(CPPFILES:.cpp=.o) $(CFILES:.c=.o) $(SFILES:.s=.o)
export OFILES := $(OFILES_SOURCES)
export HFILES :=

export INCLUDE := $(foreach dir,$(INCLUDES),-iquote $(CURDIR)/$(dir)) \
                  $(foreach dir,$(LIBDIRS),-I$(dir)/include) \
                  -I$(CURDIR)/$(BUILD)
export LIBPATHS := $(foreach dir,$(LIBDIRS),-L$(dir)/lib)

.PHONY: all $(BUILD)

all: $(BUILD)

$(BUILD): $(ARM7_ELF)
	@mkdir -p $@
	@$(MAKE) --no-print-directory -C $(BUILD) -f $(CURDIR)/Makefile

$(ARM7_ELF):
	@$(MAKE) --no-print-directory -C $(TOPDIR)/arm7 TARGET=$(ARM7_TARGET)

else

.PHONY: all

all: $(OUTPUT).nds

$(OUTPUT).nds: $(OUTPUT).elf $(ARM7_ELF)
$(OUTPUT).elf: $(OFILES)

$(OFILES_SOURCES): $(HFILES)

-include $(DEPSDIR)/*.d

endif

endif
