#!/bin/bash
# This runs cargo test and creates test coverage data, as well as a test result report, currently this requires the 'nightly' rust toolchain.
# Run this in the project root, and cargo2junit and grcov binaries (do `cargo install cargo2junit grcov`)
# Result files will be placed in ./reports

PROJECT_NAME_UNDERSCORE="uprotocol_sdk"
RUSTFLAGS="--cfg uuid_unstable -Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort"
RUSTDOCFLAGS="-Cpanic=abort"
TEMP=`mktemp --directory`


mkdir -p $TEMP/reports

cargo test $CARGO_OPTIONS -- -Z unstable-options --format json | cargo2junit > $TEMP/results.xml
zip -0 $TEMP/ccov.zip `find . \( -name "$PROJECT_NAME_UNDERSCORE*.gc*" \) -print`
grcov $TEMP/ccov.zip -s . -t lcov --llvm --ignore-not-existing --ignore "/*" --ignore "tests/*" --ignore "target/*" -o $TEMP/lcov.info
genhtml $TEMP/lcov.info --output-directory $TEMP/out

rm $TEMP/ccov.zip
mv -r $TEMP/* ./reports
rm -fr $TEMP