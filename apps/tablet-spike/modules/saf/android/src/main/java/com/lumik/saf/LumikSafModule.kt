package com.lumik.saf

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.provider.DocumentsContract
import android.provider.DocumentsContract.Document
import expo.modules.kotlin.Promise
import expo.modules.kotlin.exception.CodedException
import expo.modules.kotlin.modules.Module
import expo.modules.kotlin.modules.ModuleDefinition

// Acceso a carpetas por Storage Access Framework (experimento 2.5 del spike).
// La app elige la carpeta del disco una vez y el permiso queda guardado; Rust
// lee y escribe por fd (`openFd`), no por ruta.
class LumikSafModule : Module() {
  private var pendingPick: Promise? = null

  private val resolver
    get() = requireNotNull(appContext.reactContext).contentResolver

  override fun definition() = ModuleDefinition {
    Name("LumikSaf")

    // Abre el selector de carpetas del sistema. Devuelve la URI del árbol, o
    // null si se cancela, y guarda el permiso para que sobreviva al reinicio.
    AsyncFunction("pickDirectory") { promise: Promise ->
      if (pendingPick != null) {
        promise.reject(CodedException("E_BUSY", "Ya hay un selector abierto", null))
        return@AsyncFunction
      }
      pendingPick = promise
      val intent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE).addFlags(
        Intent.FLAG_GRANT_READ_URI_PERMISSION or
          Intent.FLAG_GRANT_WRITE_URI_PERMISSION or
          Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION or
          Intent.FLAG_GRANT_PREFIX_URI_PERMISSION
      )
      appContext.throwingActivity.startActivityForResult(intent, PICK_TREE)
    }

    OnActivityResult { _, payload ->
      if (payload.requestCode != PICK_TREE) return@OnActivityResult
      val promise = pendingPick ?: return@OnActivityResult
      pendingPick = null
      val uri = payload.data?.data
      if (payload.resultCode != Activity.RESULT_OK || uri == null) {
        promise.resolve(null)
        return@OnActivityResult
      }
      resolver.takePersistableUriPermission(
        uri,
        Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION
      )
      promise.resolve(uri.toString())
    }

    // Árboles con permiso guardado (lectura y escritura).
    Function("persistedTrees") {
      resolver.persistedUriPermissions
        .filter { it.isReadPermission && it.isWritePermission }
        .map { it.uri.toString() }
    }

    // Documento raíz del árbol, para empezar a listar.
    Function("rootDocument") { treeUri: String ->
      val tree = Uri.parse(treeUri)
      DocumentsContract.buildDocumentUriUsingTree(tree, DocumentsContract.getTreeDocumentId(tree)).toString()
    }

    // Hijos de un documento-carpeta (URI de documento dentro del árbol).
    AsyncFunction("listChildren") { documentUri: String -> children(documentUri) }

    // Igual, sin promesa: para el `DiskFiles` de Rust, que es síncrono.
    Function("listChildrenSync") { documentUri: String -> children(documentUri) }

    // Documento en `rel` (`_media/x.raf`) bajo `baseUri`, exista o no.
    Function("childUri") { baseUri: String, rel: String -> childUri(baseUri, rel) }

    Function("exists") { uri: String ->
      try {
        resolver.query(Uri.parse(uri), arrayOf(Document.COLUMN_DOCUMENT_ID), null, null, null)?.use { it.count > 0 } ?: false
      } catch (e: Exception) {
        false
      }
    }

    Function("createFile") { parentUri: String, name: String, mime: String ->
      DocumentsContract.createDocument(resolver, Uri.parse(parentUri), mime, name)?.toString()
        ?: throw CodedException("E_CREATE", "No se pudo crear $name", null)
    }

    Function("createDirectory") { parentUri: String, name: String ->
      DocumentsContract.createDocument(resolver, Uri.parse(parentUri), Document.MIME_TYPE_DIR, name)?.toString()
        ?: throw CodedException("E_CREATE", "No se pudo crear la carpeta $name", null)
    }

    // fd del documento, ya despegado de Java: quien lo recibe lo cierra.
    // `mode`: "r", "w" (trunca) o "rw".
    Function("openFd") { uri: String, mode: String ->
      val pfd = resolver.openFileDescriptor(Uri.parse(uri), if (mode == "w") "wt" else mode)
        ?: throw CodedException("E_OPEN", "No se pudo abrir $uri", null)
      pfd.detachFd()
    }

    Function("rename") { uri: String, name: String ->
      DocumentsContract.renameDocument(resolver, Uri.parse(uri), name)?.toString()
        ?: throw CodedException("E_RENAME", "No se pudo renombrar a $name", null)
    }

    // Mueve un documento a otra carpeta (mismo nombre). Devuelve su nueva URI.
    Function("move") { uri: String, fromParentUri: String, toParentUri: String ->
      DocumentsContract.moveDocument(resolver, Uri.parse(uri), Uri.parse(fromParentUri), Uri.parse(toParentUri))?.toString()
        ?: throw CodedException("E_MOVE", "No se pudo mover $uri", null)
    }

    Function("delete") { uri: String ->
      DocumentsContract.deleteDocument(resolver, Uri.parse(uri))
    }
  }

  // Con ExternalStorageProvider (memoria interna y discos USB) el id de
  // documento es la ruta (`primary:Lumik/…`, `FB5F-B3EF:lumik/…`): la URI de
  // un hijo se arma sin listar carpeta por carpeta.
  private fun childUri(baseUri: String, rel: String): String {
    val base = Uri.parse(baseUri)
    val baseId = DocumentsContract.getDocumentId(base)
    val clean = rel.trim('/')
    val id = when {
      clean.isEmpty() -> baseId
      baseId.endsWith(":") -> "$baseId$clean"
      else -> "$baseId/$clean"
    }
    return DocumentsContract.buildDocumentUriUsingTree(base, id).toString()
  }

  private fun children(documentUri: String): List<Map<String, Any?>> {
    val doc = Uri.parse(documentUri)
    val children = DocumentsContract.buildChildDocumentsUriUsingTree(doc, DocumentsContract.getDocumentId(doc))
    val columns = arrayOf(Document.COLUMN_DOCUMENT_ID, Document.COLUMN_DISPLAY_NAME, Document.COLUMN_MIME_TYPE, Document.COLUMN_SIZE)
    val out = mutableListOf<Map<String, Any?>>()
    resolver.query(children, columns, null, null, null)?.use { c ->
      while (c.moveToNext()) {
        val id = c.getString(0)
        val mime = c.getString(2)
        out.add(
          mapOf(
            "uri" to DocumentsContract.buildDocumentUriUsingTree(doc, id).toString(),
            "name" to c.getString(1),
            "isDirectory" to (mime == Document.MIME_TYPE_DIR),
            "size" to (if (c.isNull(3)) 0.0 else c.getLong(3).toDouble()),
          )
        )
      }
    }
    return out
  }

  companion object {
    private const val PICK_TREE = 4271
  }
}
