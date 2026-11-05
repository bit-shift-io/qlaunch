import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

// The shared test-phone key is committed at android/keystore/ so every PC signs
// identically; `android/keystore.properties` supplies it.
val keystoreProperties = Properties().apply {
    val file = rootProject.file("keystore.properties")
    if (file.exists()) file.inputStream().use { load(it) }
}
val hasReleaseKey = keystoreProperties.getProperty("storeFile") != null

android {
    namespace = "bitshift.qlaunch"
    compileSdk = 36
    ndkVersion = "28.2.13676358"

    // Supplied by scripts/build.sh from Cargo's resolved pinned dependency.
    sourceSets.getByName("main").java.setSrcDirs(listOf(
        "src/main/kotlin", providers.gradleProperty("icedAndroidJava").get()
    ))

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        // `-PqlaunchAppId=...` installs a side-by-side build for device
        // diagnostics without touching the installed app or its data.
        applicationId = providers.gradleProperty("qlaunchAppId").getOrElse("bitshift.qlaunch")
        minSdk = 34
        targetSdk = 36
        versionCode = 1
        versionName = "1.0.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        ndk { abiFilters += "arm64-v8a" }
    }

    signingConfigs {
        if (hasReleaseKey) {
            create("release") {
                storeFile = rootProject.file(keystoreProperties.getProperty("storeFile"))
                storePassword = keystoreProperties.getProperty("storePassword")
                keyAlias = keystoreProperties.getProperty("keyAlias")
                keyPassword = keystoreProperties.getProperty("keyPassword")
            }
        }
    }

    // Debug and release share the committed test-phone key so a debug install
    // updates an existing build in place instead of failing on a signature
    // mismatch. A missing keystore.properties falls back to the local debug key.
    val signing = if (hasReleaseKey) {
        signingConfigs.getByName("release")
    } else {
        signingConfigs.getByName("debug")
    }

    buildTypes {
        debug {
            signingConfig = signing
        }
        release {
            signingConfig = signing
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

dependencies {
    implementation("androidx.customview:customview:1.1.0")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test:runner:1.6.2")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test.uiautomator:uiautomator:2.3.0")
}
