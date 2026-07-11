import "@testing-library/jest-dom";

class ResizeObserverMock {
	observe() {}
	unobserve() {}
	disconnect() {}
}

if (!("ResizeObserver" in globalThis)) {
	Object.defineProperty(globalThis, "ResizeObserver", {
		writable: true,
		configurable: true,
		value: ResizeObserverMock,
	});
}
