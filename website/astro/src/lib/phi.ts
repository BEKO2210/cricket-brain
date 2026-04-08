export const PHI = (1 + Math.sqrt(5)) / 2;

export const phiPow = (exponent: number): number => Math.pow(PHI, exponent);

export const phiSpace = (base: number, multiplier: number): number => Number((base * phiPow(multiplier)).toFixed(4));
