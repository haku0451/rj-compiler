package com.github.haku0451.rj

import java.io.File

object RjRuntimePackaging {
    fun pluginJar(): File? {
        val location = RjRuntimePackaging::class.java.protectionDomain?.codeSource?.location
            ?: return null
        val file = File(location.toURI())
        return file.takeIf { it.isFile && it.name.endsWith(".jar") }
    }
}
