#!/usr/bin/env bash
# ADR-033 E1: validate the MusicXML export against the MusicXML 4.0 XSD.
#
# The XSD files are fetched from w3c/musicxml at the v4.0 tag (pinned commit) and
# checked by SHA-256. They are not stored in this repository: the licence (W3C
# Community Final Specification Agreement) has not been reviewed (ADR-033 choice 5.6).
# musicxml.xsd imports xml.xsd and xlink.xsd from http://www.musicxml.org/xsd/; an
# XML catalog maps them to the verified copies and xmllint runs with --nonet, so
# nothing unverified is read.
#
# Usage (from the repository root, Linux, needs curl, sha256sum, xmllint and cargo):
#   tools/musicxml-schema/validate.sh
set -euo pipefail

COMMIT=799e2defb2ece0ae7bafe08dcbcac25b2c631d53
BASE="https://raw.githubusercontent.com/w3c/musicxml/${COMMIT}/schema"
WORK="${WORK:-target/musicxml-schema}"
XSD="$WORK/xsd"
OUT="$WORK/out"
mkdir -p "$XSD" "$OUT"

for f in musicxml.xsd xml.xsd xlink.xsd; do
  curl --fail --silent --show-error --location --retry 3 -o "$XSD/$f" "$BASE/$f"
done
(
  cd "$XSD"
  sha256sum --check --strict <<'EOF'
bfe37ed25a9ec00e6f2591d53df260b84efe12aed209ba3ac0a76f9287665a99  musicxml.xsd
616a3077df5cfc954ac74a75abe9697b95eef7a85dbe09367d995a483e840eb5  xml.xsd
6e601f8eeb41618b50e4c7f944dff754e57ea43b602755470dda24c9c2f6df92  xlink.xsd
EOF
)

ABS="$(cd "$XSD" && pwd)"
cat > "$XSD/catalog.xml" <<EOF
<?xml version="1.0"?>
<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
  <system systemId="http://www.musicxml.org/xsd/xml.xsd" uri="file://$ABS/xml.xsd"/>
  <uri name="http://www.musicxml.org/xsd/xml.xsd" uri="file://$ABS/xml.xsd"/>
  <system systemId="http://www.musicxml.org/xsd/xlink.xsd" uri="file://$ABS/xlink.xsd"/>
  <uri name="http://www.musicxml.org/xsd/xlink.xsd" uri="file://$ABS/xlink.xsd"/>
</catalog>
EOF

# The E1 inputs: three reference scores, an empty document, the ADR-035 techniques
# score (TQ-7), and a MIDI-imported score.
cargo build -p mc-cli --locked --quiet
CLI=target/debug/mc-cli
for f in feature-rich small typical empty techniques; do
  "$CLI" export-musicxml "fixture:$f" "$OUT/$f.musicxml"
done
"$CLI" export-midi fixture:feature-rich "$OUT/feature-rich.mid"
"$CLI" export-musicxml "midi:$OUT/feature-rich.mid" "$OUT/from-midi.musicxml"

# The committed goldens too, so a golden can never drift from the schema.
cp crates/mc-musicxml/tests/golden/*.musicxml "$OUT/"

validate() {
  XML_CATALOG_FILES="$XSD/catalog.xml" xmllint --noout --nonet \
    --schema "$XSD/musicxml.xsd" "$@"
}

# Negative control: a copy with an element the schema does not allow must be
# rejected, so a green run cannot come from a validator that checks nothing.
mkdir -p "$WORK/negative"
sed 's|<measure number="1">|<measure number="1"><not-musicxml/>|' \
  "$OUT/feature-rich.musicxml" > "$WORK/negative/invalid.musicxml"
grep -q '<not-musicxml/>' "$WORK/negative/invalid.musicxml"
if validate "$WORK/negative/invalid.musicxml" 2>/dev/null; then
  echo "negative control failed: an invalid file passed validation" >&2
  exit 1
fi
echo "negative control: the invalid file was rejected"

validate "$OUT"/*.musicxml
echo "MusicXML 4.0 schema: all $(ls "$OUT"/*.musicxml | wc -l) files valid"
