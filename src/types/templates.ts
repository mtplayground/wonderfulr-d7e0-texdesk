export type Template = {
  id: string;
  name: string;
  description: string;
  category: string;
  mainFileName: string;
  body: string;
  bibliography: string | null;
  isDefault: boolean;
  createdAt: string;
  updatedAt: string;
};
