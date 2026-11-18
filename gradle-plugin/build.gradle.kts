plugins {
    `kotlin-dsl`
    `java-gradle-plugin`
    `maven-publish`
}

group = "com.github.haku0451"
version = providers.gradleProperty("version").orElse("0.1.0").get()

repositories {
    mavenCentral()
}

val compileRjRuntime by tasks.registering(JavaCompile::class) {
    setSource(fileTree(layout.projectDirectory.dir("../runtime/src/main/java")))
    destinationDirectory.set(layout.buildDirectory.dir("rj-runtime/classes"))
    classpath = files()
    options.release.set(8)
}

gradlePlugin {
    plugins {
        create("rj") {
            id = "com.github.haku0451.rj"
            implementationClass = "com.github.haku0451.rj.RjPlugin"
            displayName = "RJ Language Plugin"
            description = "Compile RJ sources and package executable JARs"
        }
    }
}

tasks.named<Jar>("jar") {
    dependsOn(compileRjRuntime)
    from(compileRjRuntime.flatMap { it.destinationDirectory })
}

tasks.test {
    useJUnitPlatform()
}

publishing {
    repositories {
        maven {
            name = "GitHubPackages"
            url = uri(
                "https://maven.pkg.github.com/" +
                    providers.gradleProperty("github.repository")
                        .orElse("haku0451/rj-compiler")
                        .get()
            )

            credentials {
                username = System.getenv("GITHUB_ACTOR") ?: ""
                password = System.getenv("GITHUB_TOKEN") ?: ""
            }
        }
    }
}

