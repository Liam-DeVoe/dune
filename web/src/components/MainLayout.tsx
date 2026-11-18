// Pages/MainLayout.razor: the shell every page renders in.
import type { ReactNode } from "react";
import "./MainLayout.css";

export function MainLayout({ children }: { children: ReactNode }) {
  return (
    <div className="page">
      <div className="page__main">
        <div className="page__content">{children}</div>
      </div>
    </div>
  );
}
