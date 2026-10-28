package com.lumik.saf

import android.app.Activity
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.os.storage.StorageManager
import android.provider.Settings
import android.provider.DocumentsContract
import android.provider.DocumentsContract.Document
import android.provider.OpenableColumns
import expo.modules.kotlin.Promise
import expo.modules.kotlin.exception.CodedException
import expo.modules.kotlin.modules.Module
import expo.modules.kotlin.modules.ModuleDefinition

// Acceso a carpetas por Storage Access Framework (experimento 2.5 del spike).
// La app elige la carpeta del disco una vez y el permiso queda guardado; Rust
// lee y escribe por fd (`openFd`), no por ruta.
class LumikSafModule : Module() {
  private var pendingPick: Promise? = null
  private var pendingFiles: Promise? = null
  private var volumeReceiver: BroadcastReceiver? = null

  private val resolver
    get() = requireNotNull(appContext.reactContext).contentResolver

  override fun definition() = ModuleDefinition {
    Name("LumikSaf")

    // Se conectó o se quitó un disco: la app vuelve a mirar cuáles hay.
    Events("onVolumesChanged")

    OnCreate {
      val receiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
          sendEvent("onVolumesChanged", mapOf("action" to intent.action))
        }
      }
      val filter = IntentFilter().apply {
        addAction(Intent.ACTION_MEDIA_MOUNTED)
        addAction(Intent.ACTION_MEDIA_UNMOUNTED)
        addAction(Intent.ACTION_MEDIA_EJECT)
        addAction(Intent.ACTION_MEDIA_REMOVED)
        addAction(Intent.ACTION_MEDIA_BAD_REMOVAL)
        addDataScheme("file")
      }
      requireNotNull(appContext.reactContext).registerReceiver(receiver, filter)
      volumeReceiver = receiver
    }

    OnDestroy {
      volumeReceiver?.let { appContext.reactContext?.unregisterReceiver(it) }
      volumeReceiver = null
    }

    // Discos externos montados (USB, tarjeta): uuid, nombre y si son de solo
    // lectura. La memoria interna no cuenta.
    Function("volumes") {
      val context = requireNotNull(appContext.reactContext)
      val storage = context.getSystemService(StorageManager::class.java)
      storage.storageVolumes
        .filter { !it.isPrimary && it.uuid != null }
        .filter { it.state == Environment.MEDIA_MOUNTED || it.state == Environment.MEDIA_MOUNTED_READ_ONLY }
        .map {
          mapOf(
            "uuid" to it.uuid,
            "label" to it.getDescription(context),
            "readOnly" to (it.state == Environment.MEDIA_MOUNTED_READ_ONLY),
          )
        }
    }

    // Pide acceso a un disco en un toque: el selector abre ya en su raíz.
    // Devuelve la URI del árbol o null si se cancela (el permiso se guarda).
    AsyncFunction("requestVolumeAccess") { uuid: String, promise: Promise ->
      if (pendingPick != null) {
        promise.reject(CodedException("E_BUSY", "Ya hay un selector abierto", null))
        return@AsyncFunction
      }
      val context = requireNotNull(appContext.reactContext)
      val volume = context.getSystemService(StorageManager::class.java).storageVolumes.find { it.uuid == uuid }
      if (volume == null) {
        promise.reject(CodedException("E_NO_VOLUME", "El disco $uuid ya no está conectado", null))
        return@AsyncFunction
      }
      val intent = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
        volume.createOpenDocumentTreeIntent()
      } else {
        Intent(Intent.ACTION_OPEN_DOCUMENT_TREE)
      }
      intent.addFlags(
        Intent.FLAG_GRANT_READ_URI_PERMISSION or
          Intent.FLAG_GRANT_WRITE_URI_PERMISSION or
          Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION or
          Intent.FLAG_GRANT_PREFIX_URI_PERMISSION
      )
      pendingPick = promise
      appContext.throwingActivity.startActivityForResult(intent, PICK_TREE)
    }

    // Abre una carpeta en la app de Archivos del sistema (como "Abrir carpeta"
    // del desktop). Se pasa como documento suelto, sin el árbol del permiso,
    // que es como la reconoce la app de Archivos. false si ninguna app la abre.
    Function("openFolder") { documentUri: String ->
      val doc = Uri.parse(documentUri)
      val plain = DocumentsContract.buildDocumentUri(doc.authority, DocumentsContract.getDocumentId(doc))
      val intent = Intent(Intent.ACTION_VIEW)
        .setDataAndType(plain, Document.MIME_TYPE_DIR)
        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION)
      try {
        requireNotNull(appContext.reactContext).startActivity(intent)
        true
      } catch (e: android.content.ActivityNotFoundException) {
        false
      }
    }

    // Una app no puede desmontar un disco: se abre la pantalla de
    // almacenamiento de Android, donde está "Expulsar".
    Function("openStorageSettings") {
      val intent = Intent(Settings.ACTION_INTERNAL_STORAGE_SETTINGS).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
      requireNotNull(appContext.reactContext).startActivity(intent)
    }

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

    // Selector de archivos del sistema, con selección múltiple (como el wizard
    // del desktop). Sin filtro de tipo: así Android abre el explorador completo
    // y no el selector de fotos, que no muestra los RAW. Devuelve nombre y tamaño
    // de cada uno; el permiso de lectura dura mientras viva la app.
    AsyncFunction("pickFiles") { promise: Promise ->
      if (pendingFiles != null) {
        promise.reject(CodedException("E_BUSY", "Ya hay un selector abierto", null))
        return@AsyncFunction
      }
      pendingFiles = promise
      val intent = Intent(Intent.ACTION_OPEN_DOCUMENT)
        .addCategory(Intent.CATEGORY_OPENABLE)
        .setType("*/*")
        .putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true)
        .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
      appContext.throwingActivity.startActivityForResult(intent, PICK_FILES)
    }

    OnActivityResult { _, payload ->
      if (payload.requestCode == PICK_FILES) {
        val promise = pendingFiles ?: return@OnActivityResult
        pendingFiles = null
        val data = payload.data
        if (payload.resultCode != Activity.RESULT_OK || data == null) {
          promise.resolve(emptyList<Map<String, Any?>>())
          return@OnActivityResult
        }
        val uris = mutableListOf<Uri>()
        val clip = data.clipData
        if (clip != null) {
          for (i in 0 until clip.itemCount) uris.add(clip.getItemAt(i).uri)
        } else {
          data.data?.let { uris.add(it) }
        }
        promise.resolve(uris.map { describe(it) })
        return@OnActivityResult
      }
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

  private fun describe(uri: Uri): Map<String, Any?> {
    var name = uri.lastPathSegment ?: "archivo"
    var size = 0.0
    resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE), null, null, null)?.use { c ->
      if (c.moveToFirst()) {
        if (!c.isNull(0)) name = c.getString(0)
        if (!c.isNull(1)) size = c.getLong(1).toDouble()
      }
    }
    return mapOf("uri" to uri.toString(), "name" to name, "isDirectory" to false, "size" to size)
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
    private const val PICK_FILES = 4272
  }
}
