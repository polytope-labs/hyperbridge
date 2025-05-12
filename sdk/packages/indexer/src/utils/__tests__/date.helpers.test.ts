import { timestampToDate } from "../date.helpers";
describe('timestampToDate', () => {
  it('should convert a Unix timestamp in seconds to a Date object', () => {
    // Unix timestamp in seconds
    const result = timestampToDate(1700000000);
    expect(result).toBeInstanceOf(Date);
    expect(result.getTime()).toBe(1700000000 * 1000);
  });

  it('should convert a Unix timestamp in milliseconds to a Date object', () => {
    // Unix timestamp in milliseconds
    const result = timestampToDate(1700000000000);
    expect(result).toBeInstanceOf(Date);
    expect(result.getTime()).toBe(1700000000000);
  });

  it('should convert a BigInt timestamp to a Date object', () => {
    // BigInt timestamp
    const result = timestampToDate(1700000000n);
    expect(result).toBeInstanceOf(Date);
    expect(result.getTime()).toBe(1700000000 * 1000);
  });

  it('should handle invalid inputs gracefully', () => {
    // @ts-expect-error - testing invalid input
    expect(() => timestampToDate('not a timestamp')).toThrow();
    // @ts-expect-error - testing invalid input
    expect(() => timestampToDate(null)).toThrow();
    // @ts-expect-error - testing invalid input
    expect(() => timestampToDate(undefined)).toThrow();
  });
});
