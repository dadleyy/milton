ELM=elm
NPM=npm

VERSION=dev
API_ROOT=/api
LOGIN_URL=http://0.0.0.0:8081/auth/start
LOGOUT_URL=http://0.0.0.0:8081/auth/end
UI_ROOT=/
ASSET_ROOT=/

DEBUG_BUILD_DIR=target/debug
RELEASE_BUILD_DIR=target/release

ELM_SOURCES=$(wildcard src/Main.elm src/*.elm)

DEBUG_MAIN=$(DEBUG_BUILD_DIR)/main.js
DEBUG_BOOT=$(DEBUG_BUILD_DIR)/boot.js
DEBUG_CSS=$(DEBUG_BUILD_DIR)/main.css
DEBUG_HTML=$(DEBUG_BUILD_DIR)/index.html

RELEASE_MAIN=$(subst $(DEBUG_BUILD_DIR),$(RELEASE_BUILD_DIR),$(DEBUG_MAIN))
RELEASE_BOOT=$(subst $(DEBUG_BUILD_DIR),$(RELEASE_BUILD_DIR),$(DEBUG_BOOT))
RELEASE_CSS=$(subst $(DEBUG_BUILD_DIR),$(RELEASE_BUILD_DIR),$(DEBUG_CSS))
RELEASE_HTML=$(subst $(DEBUG_BUILD_DIR),$(RELEASE_BUILD_DIR),$(DEBUG_HTML))

.PHONY: all debug clean fmt test release

all: debug

debug: $(DEBUG_MAIN) $(DEBUG_CSS) $(DEBUG_HTML) $(DEBUG_BOOT)

release: $(RELEASE_MAIN) $(RELEASE_BOOT) $(RELEASE_CSS) $(RELEASE_HTML)

$(DEBUG_MAIN): $(ELM_SOURCES)
	mkdir -p $(DEBUG_BUILD_DIR)
	$(ELM) make $< --output $@

$(DEBUG_CSS): src/main.css $(ELM_SOURCES)
	$(NPM) run tailwind -- --input $< --output $@

$(DEBUG_HTML): src/index.xml src/index.xslt
	xsltproc \
		--stringparam version ${VERSION} \
		--stringparam apiRoot ${API_ROOT}  \
		--stringparam loginURL ${LOGIN_URL}  \
		--stringparam logoutURL ${LOGOUT_URL}  \
		--stringparam uiRoot ${UI_ROOT}  \
		--stringparam assetRoot ${ASSET_ROOT}  \
		-o $@ $<

$(DEBUG_BOOT): src/boot.ts tsconfig.json $(wildcard @types/*.ts)
	$(NPM) run tsc -- -p tsconfig.json --outDir $(DEBUG_BUILD_DIR)

$(RELEASE_MAIN): $(ELM_SOURCES)
	mkdir -p $(RELEASE_BUILD_DIR)
	$(ELM) make $< --output $(subst .js,.tmp.js,$@) --optimize
	$(NPM) run uglify -- $(subst .js,.tmp.js,$@) -o $@ -m
	rm $(subst .js,.tmp.js,$@)

$(RELEASE_BOOT): $(DEBUG_BOOT)
	$(NPM) run uglify -- $< -o $@ -m

$(RELEASE_HTML): $(DEBUG_HTML)
	cp $< $@

$(RELEASE_CSS): src/main.css $(ELM_SOURCES)
	$(NPM) run tailwind -- --input $< -o $@ -m

clean:
	rm -f $(DEBUG_MAIN)
	rm -f $(DEBUG_CSS)
	rm -f $(DEBUG_HTML)
	rm -f $(DEBUG_BOOT)
	rm -f $(RELEASE_MAIN)
	rm -f $(RELEASE_BOOT)
	rm -f $(RELEASE_CSS)
	rm -f $(RELEASE_HTML)

fmt:
	$(NPM) run lint:elm -- --yes
	$(NPM) run lint:js-pretty -- -w ./src

test:
	$(NPM) run lint:elm -- --yes
	$(NPM) run lint:js -- -c .eslintrc.js ./src
	$(NPM) run lint:js-pretty -- --check ./src
