.SUFFIXES:

ifeq ($(strip $(DEVKITARM)),)
$(error "Please set DEVKITARM in your environment. export DEVKITARM=<path to>devkitARM")
endif

include $(DEVKITARM)/ds_rules

TARGET ?= ds-controller-arm7
ARM7_TOPDIR ?= $(CURDIR)
BUILD := $(abspath $(ARM7_TOPDIR)/../build/arm7)
SOURCES := source
INCLUDES := source ../common

ARCH := -march=armv4t -mtune=arm7tdmi -mthumb

CFLAGS := -std=gnu11 -g -Wall -O2 -ffunction-sections -fdata-sections $(ARCH) $(INCLUDE) -DARM7
CXXFLAGS := $(CFLAGS) -fno-rtti -fno-exceptions
ASFLAGS := -g $(ARCH)
LDFLAGS = -specs=ds_arm7.specs -g $(ARCH) -Wl,-Map,$(notdir $*).map

LIBS := -lnds7
LIBDIRS := $(LIBNDS)

ifneq ($(BUILD),$(CURDIR))

export ARM7_TOPDIR := $(CURDIR)
export ARM7ELF := $(BUILD)/$(TARGET).elf
export VPATH := $(foreach dir,$(SOURCES),$(CURDIR)/$(dir))
export DEPSDIR := $(BUILD)

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
                  -I$(BUILD)
export LIBPATHS := $(foreach dir,$(LIBDIRS),-L$(dir)/lib)

.PHONY: all clean $(BUILD)

all: $(BUILD)

$(BUILD):
	@mkdir -p $@
	@$(MAKE) --no-print-directory -C $(BUILD) -f $(CURDIR)/Makefile

clean:
	@echo clean arm7 ...
	@rm -fr $(BUILD)

else

.PHONY: all

all: $(ARM7ELF)

$(ARM7ELF): $(OFILES)

$(OFILES_SOURCES): $(HFILES)

-include $(DEPSDIR)/*.d

endif
