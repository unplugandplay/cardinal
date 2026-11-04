## Known-answer tests for the vendored SHA-256.
## Run: nim c -r --hints:off nim/sha256_test.nim

import sha256

doAssert sha256Hex("") ==
  "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
doAssert sha256Hex("abc") ==
  "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
doAssert sha256Hex("The quick brown fox jumps over the lazy dog") ==
  "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592"

echo "sha256 ok"