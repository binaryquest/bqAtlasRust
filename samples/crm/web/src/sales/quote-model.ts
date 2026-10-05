import { formatDecimalUnits, parseDecimalUnits } from "@bqatlas/contracts";
export interface CustomerOption {
  id: string;
  code: string;
  name: string;
  active: boolean;
}
export interface QuoteLine {
  id: string;
  description: string;
  quantity: string;
  unitPrice: string;
  total?: string;
}
export interface QuoteInput {
  customerId: string;
  date: string;
  currency: string;
  lines: QuoteLine[];
}
export interface QuoteRecord extends QuoteInput {
  id: string;
  number: string;
  customerCode: string;
  customerName: string;
  status: "draft" | "submitted";
  total: string;
  modifiedAt: string;
  submittedAt: string | null;
}
export interface QuoteSummary {
  id: string;
  number: string;
  customerName: string;
  date: string;
  currency: string;
  status: string;
  total: string;
}
export function quoteTotals(
  lines: readonly Pick<QuoteLine, "quantity" | "unitPrice">[],
): { lines: (string | null)[]; total: string | null } {
  let sum = 0n,
    valid = true;
  const amounts = lines.map((line) => {
    const quantity = parseDecimalUnits(line.quantity, 3, "1000000"),
      price = parseDecimalUnits(line.unitPrice, 4, "1000000000");
    if (quantity === null || quantity === 0n || price === null) {
      valid = false;
      return null;
    }
    const cents = (quantity * price + 50000n) / 100000n;
    sum += cents;
    return formatDecimalUnits(cents, 2);
  });
  return { lines: amounts, total: valid ? formatDecimalUnits(sum, 2) : null };
}
export function quoteInput(record: QuoteInput): QuoteInput {
  return {
    customerId: record.customerId,
    date: record.date,
    currency: record.currency,
    lines: record.lines.map((line) => ({
      id: line.id,
      description: line.description,
      quantity: line.quantity,
      unitPrice: line.unitPrice,
    })),
  };
}
export function newQuote(): QuoteRecord {
  const now = new Date();
  const date = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
  return {
    id: "",
    number: "New quote",
    customerId: "",
    customerCode: "",
    customerName: "",
    date,
    currency: "USD",
    status: "draft",
    lines: [
      {
        id: crypto.randomUUID(),
        description: "",
        quantity: "1",
        unitPrice: "0",
      },
    ],
    total: "0.00",
    modifiedAt: "",
    submittedAt: null,
  };
}

export function validateQuote(input: QuoteInput): Record<string, string[]> {
  const errors: Record<string, string[]> = {};
  if (!input.customerId) errors["customerId"] = ["Choose a customer."];
  const date = new Date(input.date + "T00:00:00Z");
  if (
    !/^\d{4}-\d{2}-\d{2}$/.test(input.date) ||
    !Number.isFinite(date.getTime()) ||
    date.toISOString().slice(0, 10) !== input.date ||
    input.date < "1900-01-01"
  )
    errors["date"] = ["Choose a valid date from 1900 onwards."];
  if (!["USD", "EUR", "BDT"].includes(input.currency))
    errors["currency"] = ["Choose USD, EUR or BDT."];
  if (input.lines.length < 1 || input.lines.length > 100)
    errors["lines"] = ["A quote needs between 1 and 100 lines."];
  input.lines.forEach((line, index) => {
    const prefix = `lines[${index}]`;
    if (!line.description.trim() || line.description.trim().length > 200)
      errors[prefix + ".description"] = [
        "Use a description of 1–200 characters.",
      ];
    const quantity = parseDecimalUnits(line.quantity, 3, "1000000");
    if (quantity === null || quantity === 0n)
      errors[prefix + ".quantity"] = [
        "Use a positive decimal up to 1000000 with at most 3 decimal places.",
      ];
    if (parseDecimalUnits(line.unitPrice, 4, "1000000000") === null)
      errors[prefix + ".unitPrice"] = [
        "Use a decimal from 0 to 1000000000 with at most 4 decimal places.",
      ];
  });
  return errors;
}

/** Catalog selection takes a snapshot; later catalog edits never rewrite a quote. */
export function appendCatalogProduct(
  quote: Pick<QuoteInput, "currency" | "lines">,
  product: {
    name: string;
    unitPrice: string;
    currency: string;
    active: boolean;
  },
): QuoteLine[] {
  if (!product.active || product.currency !== quote.currency)
    throw new Error("Choose an active product in the quote currency.");
  if (quote.lines.length >= 100)
    throw new Error("A quote supports up to 100 lines.");
  if (parseDecimalUnits(product.unitPrice, 4, "1000000000") === null)
    throw new Error("The catalog product has an invalid unit price.");
  const line = {
    id: crypto.randomUUID(),
    description: product.name,
    quantity: "1",
    unitPrice: product.unitPrice,
  };
  return quote.lines.length === 1 &&
    !quote.lines[0].description.trim() &&
    quote.lines[0].unitPrice === "0"
    ? [line]
    : [...quote.lines, line];
}
