export interface Website {
  id: number;
  name: string;
  url: string;
}

export interface WebsiteResult {
  testedAtMs: number;
  latencyMs: number | null;
  failure: string | null;
}

export interface WebsiteTests {
  revision: number;
  entries: { target: Website; result: WebsiteResult | null }[];
  pending: number[];
  busy: boolean;
  warning: string | null;
}
