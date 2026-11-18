package com.github.haku0451.rj

import org.gradle.api.GradleException
import java.io.File

object RjcLocator {
    fun resolve(rootProjectDir: File, configuredPath: String?): String {
        if (!configuredPath.isNullOrBlank()) {
            val file = File(configuredPath)
            if (file.exists()) {
                return file.absolutePath
            }
            throw GradleException("configured rjc executable not found: ${file.absolutePath}")
        }

        System.getenv("RJC_HOME")?.let { home ->
            val candidate = File(home, binName())
            if (candidate.exists()) {
                return candidate.absolutePath
            }
        }

        findOnPath()?.let { return it }

        RjcBundler.resolve()?.let { return it }

        findInMonorepo(rootProjectDir)?.let { return it }

        throw GradleException(
            """
            rjc compiler not found for platform ${RjcBundler.platformKey()}. Try one of:
              - rj { rjcExecutable = "/path/to/rjc" }
              - export RJC_HOME=/path/to/rjc/install
              - add rjc to PATH
              - use a Gradle plugin release that bundles rjc for your OS/arch
              - cargo build --release (in rjc repo root, for local development)
            """.trimIndent(),
        )
    }

    private fun findOnPath(): String? {
        val pathEnv = System.getenv("PATH") ?: return null
        val separator = File.pathSeparatorChar
        for (dir in pathEnv.split(separator)) {
            val candidate = File(dir, binName())
            if (candidate.exists()) {
                return candidate.absolutePath
            }
        }
        return null
    }

    private fun findInMonorepo(rootProjectDir: File): String? {
        var dir: File? = rootProjectDir
        repeat(6) {
            val current = dir ?: return null
            val release = File(current, "target/release/${binName()}")
            if (release.exists()) {
                return release.absolutePath
            }
            val debug = File(current, "target/debug/${binName()}")
            if (debug.exists()) {
                return debug.absolutePath
            }
            dir = current.parentFile
        }
        return null
    }

    private fun binName(): String = if (System.getProperty("os.name").lowercase().contains("win")) {
        "rjc.exe"
    } else {
        "rjc"
    }
}
