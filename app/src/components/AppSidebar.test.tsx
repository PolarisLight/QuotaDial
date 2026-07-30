import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, test, vi } from "vitest";
import { AppSidebar } from "./AppSidebar";

describe("AppSidebar", () => {
  test("opens settings and shows version plus yetform copyright", () => {
    const onNavigate = vi.fn();
    render(
      <AppSidebar
        destination="overview"
        version="0.1.0"
        onNavigate={onNavigate}
      />,
    );

    expect(
      screen.getByText("QuotaDial", { selector: ".brand strong" }),
    ).toBeVisible();
    expect(screen.queryByText("Quota")).not.toBeInTheDocument();
    expect(screen.queryByText("Dial")).not.toBeInTheDocument();
    expect(screen.getByText("QuotaDial v0.1.0")).toBeVisible();
    expect(screen.getByRole("link", { name: "© 2026 yetform" })).toHaveAttribute(
      "href",
      "https://yetform.cyhao.space/",
    );
    fireEvent.click(screen.getByRole("button", { name: "设置" }));
    expect(onNavigate).toHaveBeenCalledWith("settings");
  });
});
