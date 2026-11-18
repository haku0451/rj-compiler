package com.github.haku0451.rj

import org.gradle.api.Project
import org.gradle.api.provider.Property

abstract class RjExtension(project: Project) {
    val language: Property<String> = project.objects.property(String::class.java).convention("ja")
    val sourceDir: Property<String> = project.objects.property(String::class.java).convention("src/main/rj")
    val outputDir: Property<String> = project.objects.property(String::class.java).convention("build/classes/rj")
    val mainClass: Property<String> = project.objects.property(String::class.java)
    val rjcExecutable: Property<String> = project.objects.property(String::class.java)
    val jarName: Property<String> = project.objects.property(String::class.java).convention("app.jar")
}
