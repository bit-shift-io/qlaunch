# R8 rules for the release build.

# QLaunchBridge is the entire OS-facing surface of the Rust core. Rust reaches
# it only through JNI: `JNI_OnLoad` does `FindClass("bitshift/qlaunch/QLaunchBridge")`
# and every method is invoked by its exact name and signature. R8 cannot see any
# of those references — from its point of view the class is unused (its Kotlin
# callers are inlined), so the release build removes it and `System.loadLibrary`
# then fails with `ClassNotFoundException`. Keep the class and its members as-is.
-keep class bitshift.qlaunch.QLaunchBridge { *; }
