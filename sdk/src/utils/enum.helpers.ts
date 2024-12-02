/**
 * Convert a postgres formatted enumList to an Array
 * @example "{SEPOLIA, BSC}" => ['SEPOLIA', 'BSC']
 * @see https://stackoverflow.com/questions/18234946/postgresql-insert-into-an-array-of-enums
 */
export const convertEnumListStringToArray = (
  enumListString: string,
): Array<string> => {
  // Remove the wrapping brackets, e.g. {SEPOLIA, BSC} ==> SEPOLIA, BSC
  if (
    ["{", "["].includes(enumListString[0]) &&
    ["}", "]"].includes(enumListString[enumListString.length - 1])
  ) {
    enumListString = enumListString.substring(1, enumListString.length - 1);
  }

  let enumList = enumListString.split(",");
  return enumList.map((item) => item.trim());
};

/**
 * Convert an array to a  postgres formatted enumList
 * @example "[SEPOLIA, BSC]" => {'SEPOLIA', 'BSC'}
 * @see https://stackoverflow.com/questions/18234946/postgresql-insert-into-an-array-of-enums
 */
export const convertArrayToEnumListString = (
  enumsArray: Array<string>,
): string => {
  const enumListString = enumsArray.join(",");
  return `{${enumListString}}`;
};
