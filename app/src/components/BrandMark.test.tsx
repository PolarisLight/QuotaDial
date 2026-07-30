import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { BrandMark } from "./BrandMark";

describe("BrandMark", () => {
  it("renders the quota dial anatomy with an accessible name", () => {
    const { container } = render(<BrandMark />);

    expect(screen.getByRole("img", { name: "QuotaDial 额度表盘" })).toBeInTheDocument();
    expect(container.querySelector(".brand-mark__track")).toBeInTheDocument();
    expect(container.querySelector(".brand-mark__used")).toBeInTheDocument();
    expect(container.querySelectorAll(".brand-mark__tick")).toHaveLength(6);
    expect(container.querySelector(".brand-mark__hand")).toBeInTheDocument();
    expect(container.querySelector(".brand-mark__hub")).toBeInTheDocument();
    expect(container.querySelector(".brand-mark__level")).not.toBeInTheDocument();
  });
});
