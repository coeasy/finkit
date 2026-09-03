plugins {
    id("com.android.library") version "8.5.2"
}

android {
    namespace = "com.finkit.indicators"
    compileSdk = 34

    defaultConfig {
        minSdk = 24
        consumerProguardFiles("consumer-rules.pro")
    }

    sourceSets {
        getByName("main") {
            java.srcDir("src/main/java")
            manifest.srcFile("src/main/AndroidManifest.xml")
            jniLibs.srcDir("src/main/jniLibs")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
}
