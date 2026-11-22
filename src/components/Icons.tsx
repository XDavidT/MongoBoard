// Small stroke icons (inherit currentColor).
type P = { size?: number };
const base = (size: number) => ({
  width: size, height: size, viewBox: "0 0 24 24", fill: "none", stroke: "currentColor",
  strokeWidth: 2, strokeLinecap: "round" as const, strokeLinejoin: "round" as const, "aria-hidden": true,
});
export const Plus = ({ size = 12 }: P) => <svg {...base(size)} strokeWidth={2.6}><path d="M12 5v14M5 12h14" /></svg>;
export const Chevron = ({ size = 12 }: P) => <svg {...base(size)} strokeWidth={2.4}><path d="m9 6 6 6-6 6" /></svg>;
export const Search = ({ size = 13 }: P) => <svg {...base(size)}><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>;
export const Close = ({ size = 14 }: P) => <svg {...base(size)} strokeWidth={2.2}><path d="M6 6l12 12M18 6 6 18" /></svg>;
export const Check = ({ size = 14 }: P) => <svg {...base(size)} strokeWidth={2.4}><path d="m5 12 5 5L20 7" /></svg>;
export const Refresh = ({ size = 13 }: P) => <svg {...base(size)}><path d="M20 11a8 8 0 1 0-2.3 5.7M20 4v7h-7" /></svg>;
export const Database = ({ size = 14 }: P) => <svg {...base(size)}><ellipse cx="12" cy="6" rx="7" ry="3" /><path d="M5 6v12c0 1.7 3.1 3 7 3s7-1.3 7-3V6M5 12c0 1.7 3.1 3 7 3s7-1.3 7-3" /></svg>;
export const Collection = ({ size = 14 }: P) => <svg {...base(size)}><rect x="4" y="4" width="16" height="16" rx="2" /><path d="M4 10h16M10 10v10" /></svg>;
export const Link = ({ size = 13 }: P) => <svg {...base(size)}><path d="M9 7H6a4 4 0 0 0 0 8h3M15 7h3a4 4 0 0 1 0 8h-3M8 11h8" /></svg>;
export const Server = ({ size = 14 }: P) => <svg {...base(size)}><rect x="3" y="4" width="18" height="7" rx="1.5" /><rect x="3" y="13" width="18" height="7" rx="1.5" /><path d="M7 7.5h.01M7 16.5h.01" /></svg>;
export const Trash = ({ size = 13 }: P) => <svg {...base(size)}><path d="M4 7h16M10 11v6M14 11v6M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12M9 7V4h6v3" /></svg>;
