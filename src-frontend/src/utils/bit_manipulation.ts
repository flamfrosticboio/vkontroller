import type { UnsignedInt32 } from '../vkwebsocket';

export function toggleBit(
  num: number,
  mask: number,
  value: boolean,
): UnsignedInt32 {
  const bit = _toBit(value);
  return (((num & ~mask) | (bit * mask)) >>> 0) as UnsignedInt32;
}

// Inline helper that V8 will optimize cleanly to a 1 or 0
function _toBit(b: boolean): number {
  return b ? 1 : 0;
}
