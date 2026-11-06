#!/usr/bin/env bash
# Build a debug-signed APK using only the Android SDK, JDK and Rust.
set -euo pipefail
cd "$(dirname "$0")/.."
sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
bt="$sdk/build-tools/${BUILD_TOOLS_VERSION:-35.0.0}"
jar="$sdk/platforms/android-${ANDROID_PLATFORM:-36}/android.jar"
out="$PWD/build"
for tool in "$bt/aapt2" "$bt/d8" "$bt/apksigner" "$bt/zipalign" "$jar"; do
    [[ -e "$tool" ]] || { printf 'Android SDK component missing: %s\n' "$tool" >&2; exit 1; }
done
cargo build "$@" --locked --release --target wasm32-unknown-unknown
mkdir -p "$out/assets" "$out/dex"
classes="$(mktemp -d "$out/classes.XXXXXXXX")"
trap 'rm -rf -- "$classes"' EXIT
cp assets/* "$out/assets/"
cp target/wasm32-unknown-unknown/release/ecampus_mobile.wasm "$out/assets/"
"$bt/aapt2" compile --dir android/res -o "$out/res.zip"
"$bt/aapt2" link -o "$out/package.apk" -I "$jar" --manifest android/AndroidManifest.xml -A "$out/assets" "$out/res.zip"
javac -encoding UTF-8 -source 8 -target 8 -bootclasspath "$jar:$bt/core-lambda-stubs.jar" -d "$classes" android/src/ru/ncfu/ecampus/*.java
jar cf "$out/classes.jar" -C "$classes" .
"$bt/d8" --min-api 26 --lib "$jar" --output "$out/dex" "$out/classes.jar"
(cd "$out/dex"; zip -q -0 "$out/package.apk" classes.dex)
"$bt/zipalign" -f 4 "$out/package.apk" "$out/aligned.apk"
key="$out/debug.keystore"
if [[ ! -f "$key" ]]; then
    keytool -genkeypair -keystore "$key" -storepass android -keypass android -alias androiddebugkey -keyalg RSA -validity 10000 -dname "CN=eCampus Local Debug"
fi
"$bt/apksigner" sign --ks "$key" --ks-pass pass:android --out "$out/eCampus-debug.apk" "$out/aligned.apk"
"$bt/apksigner" verify "$out/eCampus-debug.apk"
printf '%s\n' "$out/eCampus-debug.apk"
