import { useEffect, useState } from 'react';
import * as api from '../../lib/api';
import type { AlbumThumbKey } from '../../lib/types';

/** Miniaturas ya pedidas en esta sesión, por clave. */
const cache = new Map<string, string>();

const keyOf = (k: AlbumThumbKey) => `${k.kind}:${k.id}`;

/** Miniatura del índice de un elemento de álbum (o de un candidato), con caché. */
export function useAlbumThumb(key: AlbumThumbKey | null): string | null {
  const [url, setUrl] = useState<string | null>(key ? cache.get(keyOf(key)) ?? null : null);
  useEffect(() => {
    if (!key) {
      setUrl(null);
      return;
    }
    const cached = cache.get(keyOf(key));
    if (cached) {
      setUrl(cached);
      return;
    }
    let active = true;
    api.getAlbumThumbnail(key).then((data) => {
      if (data) cache.set(keyOf(key), data);
      if (active) setUrl(data);
    });
    return () => { active = false; };
  }, [key?.kind, key?.id]); // eslint-disable-line react-hooks/exhaustive-deps
  return url;
}
