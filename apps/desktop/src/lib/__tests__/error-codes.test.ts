import { describe, it, expect } from "vitest";
import { ErrorCode } from "@skills-pp/shared";

describe("ErrorCode", () => {
  it("has all expected error codes", () => {
    expect(ErrorCode.DIR_NOT_FOUND).toBe("DIR_NOT_FOUND");
    expect(ErrorCode.DIR_NOT_WRITABLE).toBe("DIR_NOT_WRITABLE");
    expect(ErrorCode.NETWORK_ERROR).toBe("NETWORK_ERROR");
    expect(ErrorCode.SOURCE_FETCH_FAILED).toBe("SOURCE_FETCH_FAILED");
    expect(ErrorCode.INSTALL_CONFLICT).toBe("INSTALL_CONFLICT");
    expect(ErrorCode.INSTALL_FAILED).toBe("INSTALL_FAILED");
    expect(ErrorCode.UNINSTALL_FAILED).toBe("UNINSTALL_FAILED");
    expect(ErrorCode.DB_ERROR).toBe("DB_ERROR");
  });

  it("error codes are unique strings", () => {
    const values = Object.values(ErrorCode);
    const unique = new Set(values);
    expect(unique.size).toBe(values.length);
  });
});
