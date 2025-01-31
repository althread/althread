/** @jsxImportSource solid-js */
import { JSX } from "solid-js/jsx-runtime";

interface IProps extends JSX.SvgSVGAttributes<SVGSVGElement> {}

export const Logo = (props: IProps) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      height="100%"
      fill="none"
      viewBox="0 0 142 142"
      {...props}
    >
      <g clip-path="url(#a)">
        <path
          fill="url(#b)"
          d="M112 0H30C13.432 0 0 13.432 0 30v82c0 16.569 13.432 30 30 30h82c16.569 0 30-13.431 30-30V30c0-16.568-13.431-30-30-30"
        />
        <path
          stroke="#fff"
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="8"
          d="M114.5 90.333H95.167a4.833 4.833 0 0 0-4.834 4.834V114.5a4.833 4.833 0 0 0 4.834 4.833H114.5a4.833 4.833 0 0 0 4.833-4.833V95.167a4.833 4.833 0 0 0-4.833-4.834m-67.667 0H27.5a4.833 4.833 0 0 0-4.833 4.834V114.5a4.833 4.833 0 0 0 4.833 4.833h19.333a4.833 4.833 0 0 0 4.834-4.833V95.167a4.833 4.833 0 0 0-4.834-4.834m33.834-67.666H61.333A4.833 4.833 0 0 0 56.5 27.5v19.333a4.833 4.833 0 0 0 4.833 4.834h19.334a4.833 4.833 0 0 0 4.833-4.834V27.5a4.833 4.833 0 0 0-4.833-4.833"
        />
        <path
          stroke="#fff"
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="8"
          d="M37.167 90.333v-14.5A4.833 4.833 0 0 1 42 71h58a4.835 4.835 0 0 1 4.833 4.833v14.5M71 71V51.667"
        />
      </g>
      <defs>
        <linearGradient
          id="b"
          x1="142"
          x2="0"
          y1="0"
          y2="142"
          gradientUnits="userSpaceOnUse"
        >
          <stop stop-color="#DD8B3F" />
          <stop offset="1" stop-color="#B84F13" />
        </linearGradient>
        <clipPath id="a">
          <path fill="#fff" d="M0 0h142v142H0z" />
        </clipPath>
      </defs>
    </svg>
  );
};
