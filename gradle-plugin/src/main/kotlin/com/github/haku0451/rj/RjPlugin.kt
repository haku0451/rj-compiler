package com.github.haku0451.rj

import org.gradle.api.Plugin
import org.gradle.api.Project
import org.gradle.api.tasks.bundling.Jar
import org.gradle.kotlin.dsl.create
import org.gradle.kotlin.dsl.named
import org.gradle.kotlin.dsl.register

class RjPlugin : Plugin<Project> {
    override fun apply(project: Project) {
        project.pluginManager.apply("lifecycle-base")

        val extension = project.extensions.create<RjExtension>("rj", project)

        val compileRj = project.tasks.register<CompileRjTask>("compileRj") {
            group = "build"
            description = "Compile RJ source files to JVM bytecode"
            language.set(extension.language)
            sourceDir.set(project.layout.projectDirectory.dir(extension.sourceDir))
            outputDir.set(project.layout.projectDirectory.dir(extension.outputDir))
            rjcExecutable.set(extension.rjcExecutable)
            rootProjectDir.set(project.rootProject.layout.projectDirectory)
        }

        val rjJar = project.tasks.register<Jar>("rjJar") {
            group = "build"
            description = "Package compiled RJ classes into an executable JAR"
            dependsOn(compileRj)

            archiveFileName.set(extension.jarName)
            destinationDirectory.set(project.layout.buildDirectory.dir("libs"))

            from(compileRj.flatMap { it.outputDir }) {
                include("**/*.class")
            }

            RjRuntimePackaging.pluginJar()?.let { pluginJar ->
                from(project.zipTree(pluginJar)) {
                    include("rj/runtime/**/*.class")
                }
            }

            manifest {
                attributes(
                    mapOf("Main-Class" to extension.mainClass.map { mainClass ->
                        if (mainClass.isBlank()) {
                            throw org.gradle.api.GradleException(
                                "rj.mainClass is required for rjJar (e.g. rj { mainClass = \"com.example.Main\" })",
                            )
                        }
                        mainClass
                    }),
                )
            }
        }

        project.tasks.named("build") {
            dependsOn(rjJar)
        }
    }
}
