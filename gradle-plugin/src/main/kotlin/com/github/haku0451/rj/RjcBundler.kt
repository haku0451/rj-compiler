package com.github.haku0451.rj

import java.io.File
import java.io.InputStream
import java.security.MessageDigest

object RjcBundler {
    private const val RESOURCE_ROOT = "rjc"

    fun resolve(): String? {
        val platform = platformKey()
        val resourcePath = "$RESOURCE_ROOT/$platform/${binName()}"
        val input = RjcBundler::class.java.classLoader.getResourceAsStream(resourcePath) ?: return null

        input.use { stream ->
            val digest = sha256(stream)
            val cacheDir = File(
                File(System.getProperty("user.home"), ".gradle"),
                "rjc/$platform/$digest",
            )
            val executable = File(cacheDir, binName())
            if (!executable.isFile) {
                cacheDir.mkdirs()
                RjcBundler::class.java.classLoader.getResourceAsStream(resourcePath)!!.use { fresh ->
                    fresh.transferTo(executable.outputStream())
                }
                if (!isWindows()) {
                    executable.setExecutable(true)
                }
            }
            return executable.absolutePath
        }
    }

    fun platformKey(): String {
        val os = System.getProperty("os.name").lowercase()
        val arch = System.getProperty("os.arch").lowercase()
        val osKey = when {
            os.contains("win") -> "windows"
            os.contains("mac") || os.contains("darwin") -> "macos"
            else -> "linux"
        }
        val archKey = when (arch) {
            "amd64", "x86_64" -> "x86_64"
            "aarch64", "arm64" -> "aarch64"
            else -> arch
        }
        return "$osKey-$archKey"
    }

    private fun binName(): String = if (isWindows()) "rjc.exe" else "rjc"

    private fun isWindows(): Boolean =
        System.getProperty("os.name").lowercase().contains("win")

    private fun sha256(stream: InputStream): String {
        val digest = MessageDigest.getInstance("SHA-256")
        val buffer = ByteArray(8192)
        var read = stream.read(buffer)
        while (read >= 0) {
            if (read > 0) {
                digest.update(buffer, 0, read)
            }
            read = stream.read(buffer)
        }
        return digest.digest().joinToString("") { byte ->
            "%02x".format(byte.toInt() and 0xff)
        }
    }
}
