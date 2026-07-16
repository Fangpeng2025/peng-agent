# Add project specific ProGuard rules here.

# Rust JNI bridge - keep all native methods
-keepclassmembers class com.peng.agent.backend.BackendNative {
    *** *;
}

# Keep data classes used in JSON serialization
-keepclassmembers class com.peng.agent.client.** {
    <fields>;
}

# Chaquopy
-keep class com.chaquo.** { *; }
-dontwarn com.chaquo.**

# RapidOCR
-keep class io.github.hzkitty.** { *; }
-dontwarn io.github.hzkitty.**

# ONNX Runtime
-keep class ai.onnxruntime.** { *; }
-dontwarn ai.onnxruntime.**

# OpenCV
-keep class org.opencv.** { *; }
-dontwarn org.opencv.**
