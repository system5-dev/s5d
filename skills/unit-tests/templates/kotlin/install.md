  JUnit 5 + Kover (JetBrains, replaces JaCoCo for Kotlin DSL projects).

  build.gradle.kts:
    plugins {
        kotlin("jvm") version "2.1.0"
        id("org.jetbrains.kotlinx.kover") version "0.9.0"
    }
    dependencies {
        testImplementation(platform("org.junit:junit-bom:5.11.4"))
        testImplementation("org.junit.jupiter:junit-jupiter")
        testRuntimeOnly("org.junit.platform:junit-platform-launcher")
        // OR Kotest: testImplementation("io.kotest:kotest-runner-junit5:5.9.1")
    }
    tasks.test { useJUnitPlatform() }
    kover {
        reports {
            verify {
                rule { minBound({{THRESHOLD}}) }
            }
            total {
                xml { onCheck = true }
                html { onCheck = true }
            }
        }
    }

  Run via skill:
    bash <skill>/scripts/run.sh --stacks kotlin
