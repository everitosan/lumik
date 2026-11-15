// Marcas de fotos y elementos (spec photo-marking): reglas puras de Ctrl/Cmd+clic
// y Shift+clic sobre una lista en el orden en que se ve. Las usan el grid del
// proyecto y la sección "Papelera".

/** Marca o desmarca `id`. Devuelve un conjunto nuevo. */
export function toggleMark(marks: ReadonlySet<string>, id: string): Set<string> {
  const next = new Set(marks);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  return next;
}

/** Suma a las marcas todo lo que hay entre `anchor` y `id` en `order` (los dos
 *  incluidos). Sin ancla, o si el ancla ya no está en `order`, marca solo `id`. */
export function markRange(marks: ReadonlySet<string>, order: readonly string[], anchor: string | null, id: string): Set<string> {
  const next = new Set(marks);
  const to = order.indexOf(id);
  const from = anchor === null ? -1 : order.indexOf(anchor);
  if (to === -1) return next;
  if (from === -1) {
    next.add(id);
    return next;
  }
  const [a, b] = from <= to ? [from, to] : [to, from];
  for (let i = a; i <= b; i++) next.add(order[i]);
  return next;
}

/** Qué hace un clic en una tarjeta: marcar un rango, marcar/desmarcar, o abrir. */
export function clickIntent(e: { shiftKey: boolean }, modifier: boolean, hasMarks: boolean): 'range' | 'toggle' | 'open' {
  if (e.shiftKey) return 'range';
  if (modifier || hasMarks) return 'toggle';
  return 'open';
}
