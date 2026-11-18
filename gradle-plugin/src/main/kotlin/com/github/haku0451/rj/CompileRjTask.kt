package com.github.haku0451.rj

import java.io.ByteArrayOutputStream
import java.nio.charset.Charset
import java.nio.charset.StandardCharsets
import org.gradle.api.DefaultTask
import org.gradle.api.GradleException
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.provider.Property
import org.gradle.api.tasks.CacheableTask
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputDirectory
import org.gradle.api.tasks.Internal
import org.gradle.api.tasks.Optional
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.PathSensitivity
import org.gradle.api.tasks.TaskAction
import org.gradle.process.ExecOperations
import javax.inject.Inject

@CacheableTask
abstract class CompileRjTask @Inject constructor(
    private val execOperations: ExecOperations,
) : DefaultTask() {
    init {
        outputs.cacheIf { false }
    }

    @get:Input
    abstract val language: Property<String>

    @get:InputDirectory
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val sourceDir: DirectoryProperty

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @get:Optional
    @get:Input
    abstract val rjcExecutable: Property<String>

    @get:Internal
    abstract val rootProjectDir: DirectoryProperty

    @TaskAction
    fun compile() {
        val rjc = RjcLocator.resolve(rootProjectDir.get().asFile, rjcExecutable.orNull)
        val source = sourceDir.get().asFile
        val output = outputDir.get().asFile

        if (!source.exists()) {
            throw GradleException("RJ source directory not found: ${source.absolutePath}")
        }

        output.mkdirs()

        logger.lifecycle(
            "Running: {} build --lang {} --output {} {}",
            rjc,
            language.get(),
            output.absolutePath,
            source.absolutePath,
        )

        val stdout = ByteArrayOutputStream()
        val stderr = ByteArrayOutputStream()
        val charset = rjcOutputCharset(language.get())

        val result = execOperations.exec {
            commandLine(
                rjc,
                "build",
                "--lang",
                language.get(),
                "--output",
                output.absolutePath,
                source.absolutePath,
            )
            standardOutput = stdout
            errorOutput = stderr
            isIgnoreExitValue = true
        }

        emitLines(stdout.toString(charset)) { logger.lifecycle(it) }
        emitLines(stderr.toString(charset)) { logger.lifecycle(it) }

        if (result.exitValue != 0) {
            val err = stderr.toString(charset).trim()
            throw GradleException(
                if (err.isNotEmpty()) {
                    "rjc build failed:\n$err"
                } else {
                    "rjc build failed with exit code ${result.exitValue}"
                },
            )
        }
    }

    private fun rjcOutputCharset(lang: String): Charset {
        val isWindows = System.getProperty("os.name").lowercase().contains("windows")
        return if (isWindows && lang == "ja") {
            Charset.forName("Windows-31J")
        } else {
            StandardCharsets.UTF_8
        }
    }

    private fun emitLines(text: String, emit: (String) -> Unit) {
        text.lineSequence()
            .filter { it.isNotEmpty() }
            .forEach(emit)
    }
}
