// Cliente HTTP del portal web contra el server de respaldo (apps/server).
// Guarda los tokens de sesión y refresca de forma transparente en 401.

import { v5 as uuidv5 } from 'uuid';

/** Id determinista de un archivo exportado. Debe coincidir con el de la desktop. */
export function exportedId(projectId: string, filename: string): string {
  return uuidv5(`${projectId}/${filename}`, uuidv5.URL);
}

const ACCESS_KEY = 'lumik.access';
const REFRESH_KEY = 'lumik.refresh';

export const tokens = {
  getAccess: () => localStorage.getItem(ACCESS_KEY),
  getRefresh: () => localStorage.getItem(REFRESH_KEY),
  set(access: string, refresh: string) {
    localStorage.setItem(ACCESS_KEY, access);
    localStorage.setItem(REFRESH_KEY, refresh);
  },
  clear() {
    localStorage.removeItem(ACCESS_KEY);
    localStorage.removeItem(REFRESH_KEY);
  },
};

export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
  }
}

export interface Photographer {
  id: string;
  email: string;
  name?: string | null;
  created_at: string;
}

export interface AuthResponse {
  access: string;
  refresh: string;
  photographer: Photographer;
}

interface TokenPair {
  access: string;
  refresh: string;
}

async function parseError(res: Response): Promise<never> {
  let message = res.statusText;
  try {
    const body = await res.json();
    if (body && typeof body.error === 'string') message = body.error;
  } catch {
    /* respuesta sin cuerpo JSON */
  }
  throw new ApiError(res.status, message);
}

/** Petición autenticada: adjunta el access token y refresca una vez en 401. */
export async function request<T>(
  path: string,
  init: RequestInit = {},
  allowRefresh = true
): Promise<T> {
  const headers = new Headers(init.headers);
  // Solo JSON para cuerpos string; para FormData deja que el navegador ponga
  // el content-type con su boundary.
  if (typeof init.body === 'string' && !headers.has('content-type')) {
    headers.set('content-type', 'application/json');
  }
  const access = tokens.getAccess();
  if (access) headers.set('authorization', `Bearer ${access}`);

  const res = await fetch(path, { ...init, headers });

  if (res.status === 401 && allowRefresh && tokens.getRefresh()) {
    const ok = await tryRefresh();
    if (ok) return request<T>(path, init, false);
  }

  if (!res.ok) await parseError(res);
  if (res.status === 204) return undefined as T;
  return res.json() as Promise<T>;
}

/** Petición pública (sin token ni retry): login/register. */
async function publicPost<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(path, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!res.ok) await parseError(res);
  return res.json() as Promise<T>;
}

async function tryRefresh(): Promise<boolean> {
  const refresh = tokens.getRefresh();
  if (!refresh) return false;
  try {
    const pair = await publicPost<TokenPair>('/v1/auth/refresh', { refresh });
    tokens.set(pair.access, pair.refresh);
    return true;
  } catch {
    tokens.clear();
    return false;
  }
}

export interface Remote {
  id: string;
  provider: string;
  name: string;
  created_at: string;
}

export const providersApi = {
  list: () => request<Remote[]>('/v1/providers'),
  connect: (type: string, body: Record<string, string>) =>
    request<Remote>(`/v1/providers/${type}/connect`, {
      method: 'POST',
      body: JSON.stringify(body),
    }),
  disconnect: (id: string) =>
    request<void>(`/v1/providers/${id}`, { method: 'DELETE' }),
};

export interface CatalogProject {
  id: string;
  name: string;
  session_date: string | null;
  slug: string;
  created_at: string;
  photo_count?: number;
  backed_up?: number;
  workflow_status?: string | null;
  providers?: string | null;
}

export interface CatalogPhoto {
  id: string;
  filename: string;
  file_hash: string | null;
  jpg_path: string | null;
  backup_status: string;
  backup_url: string | null;
}

export const catalogApi = {
  listProjects: () => request<CatalogProject[]>('/v1/projects'),
  listPhotos: (projectId: string) =>
    request<CatalogPhoto[]>(`/v1/projects/${projectId}/photos`),
};

/**
 * Descarga el JPEG de portada (con auth) y devuelve un objectURL usable en <img>,
 * o `null` si el proyecto no tiene portada. Recuerda revocar el URL al desmontar.
 */
export async function getProjectCoverUrl(projectId: string): Promise<string | null> {
  const url = `/v1/projects/${projectId}/cover`;
  const withAuth = (): RequestInit => {
    const access = tokens.getAccess();
    return access ? { headers: { authorization: `Bearer ${access}` } } : {};
  };
  let res = await fetch(url, withAuth());
  if (res.status === 401 && tokens.getRefresh() && (await tryRefresh())) {
    res = await fetch(url, withAuth());
  }
  if (!res.ok) return null;
  return URL.createObjectURL(await res.blob());
}

/**
 * Backup desde el navegador/iPad. Usa el MISMO id determinista que la desktop
 * (uuid5 de `projectId/filename`), así una foto subida desde ambos lados no se
 * duplica en el catálogo ni en la nube.
 */
export const backupApi = {
  /** Crea el directorio remoto una vez, antes de subir en paralelo. */
  prepare: (projectId: string, targets?: string) =>
    request<void>(`/v1/projects/${projectId}/backup/prepare`, {
      method: 'POST',
      body: JSON.stringify({ targets: targets ?? null }),
    }),

  /**
   * Enlace expirable a la carpeta del proyecto en un proveedor. `url` es `null`
   * si el proveedor no soporta expiración (el caller abre su web).
   */
  folderLink: (projectId: string, provider: string) =>
    request<{ url: string | null }>(`/v1/projects/${projectId}/backup/link`, {
      method: 'POST',
      body: JSON.stringify({ provider }),
    }),

  /** Sube un archivo (multipart) al `_exported/` remoto del proyecto. */
  uploadFile: (
    projectId: string,
    photoId: string,
    file: File,
    targets?: string,
    quality?: string
  ) => {
    const form = new FormData();
    form.append('file', file, file.name);
    form.append('filename', file.name);
    if (targets) form.append('targets', targets);
    if (quality) form.append('quality', quality);
    return request<{ backup_status: string }>(
      `/v1/projects/${projectId}/photos/${photoId}/backup`,
      { method: 'POST', body: form }
    );
  },
};

export const authApi = {
  register: (email: string, password: string, name?: string) =>
    publicPost<AuthResponse>('/v1/auth/register', { email, password, name }),
  login: (email: string, password: string) =>
    publicPost<AuthResponse>('/v1/auth/login', { email, password }),
  me: () => request<Photographer>('/v1/auth/me'),
  logout: (refresh: string) =>
    request<void>('/v1/auth/logout', {
      method: 'POST',
      body: JSON.stringify({ refresh }),
    }),
};
