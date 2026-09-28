#!/usr/bin/env bash
# live-host check: has the real alembic cli spawn the built adapter through
# `examples/backend.yaml` and take an inventory through plan and apply.
#
# it proves the template as shipped is drivable by the pinned release: the
# backend config wires up, the process is spawned, and setup, capabilities,
# preview_schema, read, ensure_schema and write all answer well enough for the
# host to write a plan and apply it. it does not check convergence, because the
# adapter here is a skeleton whose `read` observes nothing and whose `write`
# keeps nothing -- fill in the TODOs in src/main.rs and this becomes a real
# converge test (see the sqlite/carp and python-sdk adapters for that shape).
#
# needs the alembic cli. point $ALEMBIC at it, or have `alembic` on PATH.
set -uo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"

ALEMBIC="${ALEMBIC:-alembic}"
if [ ! -x "$ALEMBIC" ]; then
  resolved="$(command -v "$ALEMBIC" 2>/dev/null || true)"
  [ -n "$resolved" ] && ALEMBIC="$resolved"
fi
if [ ! -x "$ALEMBIC" ]; then
  echo "SKIP: alembic cli not found. set \$ALEMBIC to the alembic binary."
  exit 0
fi

cargo build --release || { echo "adapter build failed"; exit 1; }
ADAPTER="$ROOT/target/release/alembic-adapter-example"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# the same backend config the readme documents, with `command` pointed at the
# binary we just built.
sed "s|^command:.*|command: $ADAPTER|" examples/backend.yaml > "$WORK/backend.yaml"

cat > "$WORK/inv.yaml" <<'EOF'
schema:
  types:
    dcim.site:
      key: { slug: { type: slug } }
      fields:
        name:   { type: string }
        slug:   { type: slug }
        status: { type: string }
    dcim.device:
      key: { name: { type: slug } }
      fields:
        name:   { type: slug }
        site:   { type: ref, target: dcim.site }
        status: { type: string }
objects:
  - uid: "a4d6a0c3-4e73-4a76-b216-4d38f8c55f3d"
    type: dcim.site
    key:   { slug: "fra1" }
    attrs: { name: "FRA1", slug: "fra1", status: "active" }
  - uid: "7b8f7a92-8fd0-4667-9a4b-9f3b5c9a4b1a"
    type: dcim.device
    key:   { name: "leaf01" }
    attrs: { name: "leaf01", site: "a4d6a0c3-4e73-4a76-b216-4d38f8c55f3d", status: "active" }
EOF

cd "$WORK"
fail=0
B=(--backend external --backend-config backend.yaml)

ops_count() { python3 -c "import json,sys; print(len(json.load(open(sys.argv[1])).get('ops',[])))" "$1"; }
expect() { # <desc> <actual> <wanted>
  if [ "$2" = "$3" ]; then echo "ok   - $1"; else echo "FAIL - $1 (got $2, wanted $3)"; fail=1; fi
}

"$ALEMBIC" validate -f inv.yaml >/dev/null || { echo "FAIL - validate"; fail=1; }

"$ALEMBIC" plan -f inv.yaml -o p1.json "${B[@]}" >/dev/null 2>&1
expect "the adapter answered read, so plan has 2 creates" "$(ops_count p1.json)" "2"

"$ALEMBIC" apply -p p1.json "${B[@]}" > apply.txt 2>&1
expect "the adapter answered write, so apply applied both ops" \
  "$(grep -c '^applied 2 operations$' apply.txt)" "1"

echo
if [ "$fail" -eq 0 ]; then echo "e2e-alembic: all checks passed"; else echo "e2e-alembic: failures above"; fi
exit "$fail"
