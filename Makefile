ORTOOLS_TAG   := v9.11
HIGHS_TAG     := v1.7.2
ORTOOLS_SRC   := vendor/ortools
HIGHS_SRC     := vendor/highs
ORTOOLS_BUILD := $(ORTOOLS_SRC)/build
HIGHS_BUILD   := $(HIGHS_SRC)/build
JOBS          := $(shell nproc 2>/dev/null || sysctl -n hw.logicalcpu)

.PHONY: all ortools highs clean distclean

all: ortools highs

ortools: $(ORTOOLS_BUILD)/lib/libortools.dylib

$(ORTOOLS_BUILD)/lib/libortools.dylib:
	@if [ ! -d $(ORTOOLS_SRC) ]; then \
	  git clone --depth 1 --branch $(ORTOOLS_TAG) \
	    https://github.com/google/or-tools $(ORTOOLS_SRC); \
	fi
	cmake -S $(ORTOOLS_SRC) -B $(ORTOOLS_BUILD) \
	  -DCMAKE_BUILD_TYPE=Release \
	  -DBUILD_DEPS=ON \
	  -DBUILD_SHARED_LIBS=ON \
	  -DBUILD_EXAMPLES=OFF \
	  -DBUILD_TESTS=OFF \
	  -DUSE_GLOP=ON \
	  -DUSE_CP_SAT=ON \
	  -DUSE_SCIP=OFF \
	  -DUSE_COINOR=OFF
	cmake --build $(ORTOOLS_BUILD) -j$(JOBS) --target ortools

highs: $(HIGHS_BUILD)/lib/libhighs.dylib

$(HIGHS_BUILD)/lib/libhighs.dylib:
	@if [ ! -d $(HIGHS_SRC) ]; then \
	  git clone --depth 1 --branch $(HIGHS_TAG) \
	    https://github.com/ERGO-Code/HiGHS $(HIGHS_SRC); \
	fi
	cmake -S $(HIGHS_SRC) -B $(HIGHS_BUILD) \
	  -DCMAKE_BUILD_TYPE=Release \
	  -DBUILD_SHARED_LIBS=ON \
	  -DFAST_BUILD=ON
	cmake --build $(HIGHS_BUILD) -j$(JOBS)

clean:
	rm -rf $(ORTOOLS_BUILD) $(HIGHS_BUILD)

distclean:
	rm -rf vendor/
