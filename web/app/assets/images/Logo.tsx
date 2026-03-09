import { JSX } from "solid-js/jsx-runtime";

import logoSrc from "./logo.png";

interface IProps extends JSX.ImgHTMLAttributes<HTMLImageElement> {}

export const Logo = (props: IProps) => {
  return (
    <img style={{height: "42px"}} src={logoSrc} alt="Althread logo" {...props} />
  );
};
