#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
compose_sdk="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
compose_ndk="$compose_sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/darwin-x86_64/bin"
export ANDROID_HOME="$compose_sdk"
export RUSTC="$(rustup which --toolchain stable rustc)"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$compose_ndk/aarch64-linux-android26-clang"
export CC_aarch64_linux_android="$compose_ndk/aarch64-linux-android26-clang"
export AR_aarch64_linux_android="$compose_ndk/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384"
rustup run stable cargo build -p compose-android-bridge --release --target aarch64-linux-android --locked
mkdir -p android/app/src/main/jniLibs/arm64-v8a dist
cp target/aarch64-linux-android/release/libcompose_android_bridge.so android/app/src/main/jniLibs/arm64-v8a/
android/gradlew -p android assembleRelease
cp android/app/build/outputs/apk/release/app-release.apk dist/Compose-Android.apk
printf 'Built %s/dist/Compose-Android.apk\n' "$PWD"
