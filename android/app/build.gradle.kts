plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "bitshift.qlaunch"
    compileSdk = 36
    ndkVersion = "28.2.13676358"

    // Supplied by scripts/build-iced.sh from Cargo's resolved pinned dependency.
    sourceSets.getByName("main").java.setSrcDirs(listOf(
        "src/main/kotlin", providers.gradleProperty("icedAndroidJava").get()
    ))

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        applicationId = "bitshift.qlaunch"
        minSdk = 34
        targetSdk = 36
        versionCode = 1
        versionName = "1.0.0-iced-spike"
        ndk { abiFilters += "arm64-v8a" }
    }

    buildTypes {
        release {
            // Test phone only; same development key as the Flutter baseline.
            signingConfig = signingConfigs.getByName("debug")
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

dependencies {
    testImplementation("junit:junit:4.13.2")
}
