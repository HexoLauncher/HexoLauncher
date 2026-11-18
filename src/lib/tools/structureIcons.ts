const ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';

interface Sprite {
	palette: string[];
	rows: string[];
}

const BLOB =
	'FGFLYsK91PXvDmsTct3k0PgcL1N+1fPk4F5sWC3c7463TyoOON760YP6R39UKtn6kKRQdQM7nvqfslJmE2rT68i2E2IANMe075fy' +
	'VjwGLt3sga9XPAJj17nOq057A2bCoszvQXgeO8OQ7s/oCzoZa8f/2r5JPgtl1v2XrwR6W3yU84n1U2tOKnqUtcnxG2NqdsezyfEC' +
	'Zx9Gya6LpktpFSHN84XgX3scBDPL5YuxGhB2WLqE+NYoHiFcuvCQq1F/Gy3rjOPJKxcUeEa31e2jRGAZI8/vmfJXCWdFuYXqv0kk' +
	'Gz/U+oO7W0N1NVKwnP7DORkDNZDvh65YYxc314msyC8AYl+h6Zeh/E9lDCDX44PDO1N2WbSX6alFew1k1f+UvEt/d1msxeD32Tlh' +
	'HTHP9danT2gIP6CN9MBrDGBPyfWBrV9nTD/b/OT7wiUCdBi6npe1SXUVJd2sk7dQG29VtZ3rjiduAyHd1feHvRhna0Gume3NKhw3' +
	'XMLvjb1PbR8phJD53z4JfVxEvdqbtlt1Bz3d45XwKw9oUKeT89NBKg8qy+eVpUl3AVwjx+CAt0NjCz2W85KxXW0JJdvtyLdTdBwr' +
	'3/+fqQJrAUw/zbfa8hJoUV6Wv8PkCDRPM8fy3qULLk94n/rUt0gxfwpsjbqW8hZ7FyXYsp76Fj9bPNLrnvhGJwpv0O+DsbQfJFdk' +
	'iP6IrldiRTSUqIC6Q28dIIvgxatNMBQ6w/bWm/pVYgk3qued4QIsDSPfmZjMOBh4TuaA5cspFWpFu43Qj6VMDX1JqYH2nTkUeVun' +
	'g+/RJiQVP7+U4Nc3F2EKr7qX9Mk0GGY1lufgxycQZES/iaLHKgdlWa2En6cMEHFRTrmO7s07U3ZQvZ7j3FVlHQC5nfraLxp5UaXP' +
	'6cQoCnQVN8eQssouDmFWpobi1HgdaESk/Y+tMwMpUbWS8cYyEgl/FLCd8rVHfXBOv9rixiUdblq9muuBJAMLJ9WO4d8pdAtvSKif' +
	'68oiFD9a0vGdwSUJd0Hkm//YMAdzUrOEq6uwUxFrV7iF850iBWJCtoLi3iwiCSqmg/zKJRtuCLeQ9OneLgxuPZjz9984AXhTqpmh' +
	'2T8aek+5mpevCg5kTqyXk//BNlNtSa+G8MZDbRUav4rnxTkFaVegzPLWMRFmN6iQ99L/DTtQZ5qM2f5TMUlnie+D4ls/BW2Bvd/y' +
	'SX1iLXHb+5m3TXwPKdG7iKJIOxA+yfmB5RhrWySE55O9RS8cXSeX+4K9TXVZMoD+wKRQZxEpg/KbvVJuHSHR6c2nRGR7CzjL5Z33' +
	'SWwBLN/jlaVdIV8snOvLqF95AWvbpZvrRRN5CTGV/J+/UGATI7z73usDNlA92ZSTxmgWclK0g/HQyzpXfVG2kYmzUQ94F6mN4N4o' +
	'H3hRrcPpziFpGymgmuexxysIb1iph+fRehV6P9Ph4Mw9CSJbtZ76zzMQd0bui/6dsSQZeEO10uLFIQF5Q6Ob6oRHZAdOuYrl1ShE' +
	'dl66qZzhyC0XVj3WmvPFOhRhV+iZ+9w2AXdftuHErygFbVFVs4f1kCMMaEi9humjWz5zTqOG+cIuE2YCs5Dy0SofFwE58pH+0TYK' +
	'dVulkbrAJwZ7T9v1m8duAGVNqZXpxTMFVm9IqI2boU8NcRq5kvneJgFpUqHO+t4ycAkzs5XiiiM+FHVOtZXm33IPaSTK8+TAPAMo' +
	'R66E7dMnCX9J5puXvqg1D2FDt97z3jETbVy1gf6AX3UcXKGa+t8nSm9Cqoj7/tgtElRUkOaA/AYtTnjP+5b6QS9CY5SznvBTbG0j' +
	'1rP63bUTPRc7yfzV6x14R2PA9o+6H2lJIIegi9ZbaVV2mMHvk9VMGG9Pp5G53DIcfUGlifTCZ3sROLCH89MzBS1KX7OR7dI9AHV4' +
	'yePkxDMHZF6ow+bKJwR5TaGfieJTF3BrXqqK6t12E3dauoX51UtzQF+7nf3KPBx3QeuO4c8tERUZKdHX6M4pA3dAo4P2nzgOY0K9' +
	'4YOzJkh3ULeV49c0LxoyV7qW9qlTcW9Z/4PmwTgPeFi7jabCJwoJO8mF+82TLgtvTLiO78cyW35S3/OBxSoWYAe7nvnRJBBzUqTN' +
	'6569UQ5xXKKVuMciBWdTpof8ymFnBCelmu3BPwkvUraTif7IKwt+dt32+9klGXZIvMH82D8fa1+/l4HkTwJvTbK2mObSdwhvS6CX' +
	'4sJDe15YroPg3CAOcEbol/PUNQJ0NyHZ0fTZNxZqXrKP+ZwjBmFbrPvk5EMmUnOIpsfUDyxbAS/TtYzpDCFAZ8y4kqckYQQ/zPfe' +
	'rUV4HifS49W1SHpuEz/N8I7jRngXMMTqk6ZQZB540PCHq1lsECnYuoKtUEZ/CjqP7JGlRnsXJdjhnqkTawg80uueqRN3DjmG7/K/' +
	'tQkqRCXL9/GgW28PL9myl6JPbREtwf+J5Fd9FDjP+5uDtR57FjvZ5ZG9Q3VcLcfslKNXdxchiu+Kp0V5BSnX7b6Pp0BgFyOshPKZ' +
	'PH4TMc3phbdPJBcz1JPk0zISZA6ovf+doV19DTWU6+XHJRJmRr6Io8ctawk1ze+fpQAUcVZIvorqyz5SczrX9YW3VWcRA7ie+d8o' +
	'HHVQocqDrkNtHxU1y5KywSgPZ1Okh+fVEncCI8/9jaEwAy1Ss5DwyTMTDxV+2/qZtUVxdkW82eXNIxxrW7v3gepDaAsl2Yrl3yd6' +
	'DG9Iq5vso0t9WjHS85HBSQl3ReKb/9hcBx8/3+XAq7JffQE90YP2mEtvCCjf64u/RyILJsvplaFNGAVg3/uchLNHawU9mv+at1Vp' +
	'FTnH8cyzV3AQJ9+AyK8KPUlhg7Kw3vcseVx5mq3G9RJjDSfbvMDvGjdbdMWw7rNVYkUnkNzq0rladgVqzrKdohxtByHMqY2rSHdG' +
	'LcbqkqBScAA4cMHrjbgde0kzl7qRsl5mEzLT5MXkVX8ZJtHlhaVTLgUeMpH6hKxTMVgpw+WV81dxDW+C6YqmVj0eMc39zL1XcXML' +
	'MN/wifZddhov3+nVuE8gETvd7Z+vTWVBarDsj/0cACgHKe3rg6RMew8vz/nSt0JvYjHN4ZOrCHsfONjvm7ujOlF0WraWhbFRYRlw' +
	'z+vm30MYf1+sx+LFKGUZKcv7gdzJR2BpUaSE7NEWGh4z0eGLqVttIze8kvvPX38fQILn8p+xQ3ERL9nd48gnB3sroPOF7ktmByvZ' +
	'4Y3cLUdwVLGrloenR3FaP9b1malVFGVV45zy3z0DEzPb7cavTG8BUlG56/mVKAZrQ7/vj69ZPhUuz4L5wUF/Zgm0mvXXK3sbAzmc' +
	'95bUNGURVsORv8IkAXcj1/ebowZvZk/FkIWpVwVTbkithJejT2kRGb6R+rFNaQU7qcv63DcYCzPd84mOK1J/HSHd8Y+5cAhgRazz' +
	'iadTZU4rxuuJtUFtEyXkkv/YqFFrByfRup+6V3UJNdnnkYg3dxxDlPGf+gBtTG6yotzZ7gg2GH2e6JjuQzFAe9mt2ORBZXgt0vLZ' +
	'rUlhVSPZ9+6O4E1uAi/J85GuTicZZ9W8j7Vbehsi2feevVVnBTragPWRuQx7AS/G5silRWoHIcvplrYQYVwtlueds1JzCjEpxuWN' +
	'v11iAG6Eop/gU2kHPtKwx+sHdxkjwf6T/xNhUx0/xeuK/QU1AHqa5Ze1Sj5VfsG82KtRfxZgnqnG6VdxexkmkKbNtQ5xFy3D4ta6' +
	'GipCNN3vjbJeO1d3lOKDuVdOYxZ3nbeGqVNxDniKpcHmTG8VO9qr1+RWLUg1x+Wa6+EIekpq2+GPphcuWCaY74GrSXZHc5zo3vRH' +
	'fRMyg7jMiPUWbR89wrPM4FUiQzPJ557oRiAaYYT5g6FeKA90xLarn6VLals3lOzcqEV3FSqb5Nj4G2oLMd/2xvgIIUg70dv5hvYH' +
	'LE9v1PeNo0IzGXzVpJC9T20SY5KwhvIQYxk3Ls/xgv5CZQkz0e7fuwR8RnnP9Zu6C2JEPY2oladFeksWaNGqzbtBbwZxjrmH/hth' +
	'CynW94WhT20EJ93zkuNKLD1Vds3/naISLkBig/KTqUd+TnSArMCzWWMBPo6+1PcWDH8FK8q6wfQQPA4l1/WK+0Q4AHqb65G/ViIL' +
	'boSumbG7WWZWZYyvz7ZXbQMikq7c7QR3HS/N8pOrTWEBJcP5l46uBG0BIpPpk7FOOEdlg6aDr1V7GmqUpJ3sWXUHJdqqocjqC3ob' +
	'Ic/mhutFZgp+weuJtgdjXCWetoe9U3JDKYydtIKtX30CI4P9kLFTcwkn3v/Zsg1yFjmy75/5QSZGMDCm6+jLPVd1Vbyd5N41DnkE' +
	'vpqcsksadVypw+HNIwBjZ1e0mrTBT2YAUaiT8sNpD3tXu4b57jsOIjfd9PLQOw8aWTORvNDxDAVSR5O8jaclF3dAmrDFrQouQm2u' +
	'kcv0TR5val62v8PnCiMJb5qu3NsTDxcvhJHt/RQvQX6EsZ778hIEW2Td7ZWiHBhAb4O/3+seMExtspTDrU9/aC+WscHcrQw6Xy/F' +
	'tOPWOmUXNdizu/tLMVp/jajH6E4bVEzN//X4yCl0Qj+FsMDuCDJUCbW7g69dBFtogOjG+B84SyfGj4KEoVp5FU2unOPNXCVBZJmr' +
	'280qBQwjyOvv3iEUd1cKuJ771iACfEybupGyOW5qVqCe75U3MWp+t5j0oEUgC315xqf76FUgDWjUh97mHQMfVt3pivIINEtvnKWK' +
	'zhgzJE5ghqfM9ksvG2uGt9L9C2cfI6X8ivxNYEF63uKBrVASOkFxjv2PpkB2GTTV58z/Tm0BPIWr1+VPMBM61OLTregecQA62fWI' +
	'+0IsDGuF5464Tm0IfNmzx6ZFaRQ717vNxrtLYgw6jvidpAxvEkDT4dXkAjBEI6b4nKtffx8pgufyn71BfREv2biJo0hoHyvLnOqB' +
	'JAlkKdXhjbNJIhM73MTz6MsrHTZTtpiVqVV5CzOA85ewUAp/WbGGrc8jYwE9OdnpkfhHYwRBs4fkwzFaeiLP7ZGhQ3MJZNv/9dIm' +
	'E3NpXPP7lrtZaRMxw/XQryYFckm8nvHMCm8KJ8n7ialXYTxuQKuM/8skBh120/6fs0FpalSix/rUMxBnUrCQ5o1HXn0RIbKd59R4' +
	'DGhAoJjmzDsITifE54m1LAB8TuqU99DMPApqJ9G2nbZXdQk1tov5hDsfd13H85O7QSQPLsPhnfTaJBI3SKyJjLtPbwM7nveCr01x' +
	'YEm4jqPWX3gYL9v34Zn+W3YbOaiZ+sQ9XQ8rzPSDu1V7AWrP6oelNAhkWsGc44egQHtwcMnxmfMTNFptk5TW6lIwWGnQ/8qzASYA' +
	'bGiFto2pJGMUatDzx7VDZwk/yLCWvFRyCy3P/4j+EGACHC3L5Yu8BWAFe8n2mbtLfFItwOyHvF9xFyCa9MTrCjF1FyfQps28WHMC' +
	'I7ztla4WO1IhxZiNv0kkGz/Y+I+7W0N1XjvW+5WnXX0DWv/vi6xUYxc31+HKr0prBzXFhvvMlilnACDX44OrXTYTMtH9jcYoFmYF' +
	'vfOUvEt/Hz/JoouQs10OcVqnkrnBT2gIP8vrk6UCaQoro5rrxj4JL1Xb/OSTp0dnHXjT+v3aIx92S73D+bdQcAcz0/eP6kNuaU63' +
	's5jn0XcOAyTM+4+jTXVSN8KG4NshDXRE6JKfuFhvFzkv1b6btjIWaVuxifqcJmsMNM/1m7dBKg8qrorzzyYYbjNKx+CMtU9jCz2W' +
	'857aPQdjSbSAobdTeB4n3/+fqQJncnZQq5f8xDRdCyPK5J+rS3MFbsvm4sI5DmBfrKCTuVBEcwcnx/Hav1oefEC8kIexAHEbMNDn' +
	'k7NbbUYjzuOBvblVaxELlOGI9gcoQG2o/9HtDCFacZzznbFXIVd6hKHByLkKDho2w/6L8llpXC3H4Yi0S2ELN4bthqpeZR5mzP6w' +
	'gatNLwg0iPyH+lFyHifW/5iiAiQVP9nkxqZAZ0Vuxd7y1elKZhQmmOmDpVVmFmfO/sKpSmYfL9vwj+8MfRcxM5z8mbpHNh022u6T' +
	'qU94XmDR+52oDW4JJ9KqgaJOLAICetbl1KVPaUYj1qOU9R51DiKFvJq2SXVISN30xO4CJm8BC9f2m7lFcXJMut/gx0x0AzfX+4Pm' +
	'T2oHJbaI490qdA5rIMD3g69JcVYz3vP+xiENc0Xgm5O0XGsTPdPpwqeyMBVoULyG9JhLbwgo0+mHs0UuC0mjhP/JIBtpYN/7nIix' +
	'S2cHMZr//NozD3hVx/HMs1d8EivT85utBg9oRKablZWrXTgHL8bgm69PbxkUs4XjwTdtAT/JpJe9VHgPO9vD9bvZMh8ZJdH9g7Uc' +
	'bQcs1OOXt1cCLkqo54W5RWkXLX7P54CgV2MDSb3V9r5TcQ0pxfeP5FdzFDzL//3eKkZlcD/d4Z29TXVUK8/oiL8pCnJF7o6Gq0l1' +
	'DS/f5cC/W3xkcUangvT6X3oXNcX3ladRLBM30JLn0zMYb2bD7oOtX1V1Cz2Y54PBKR5qSqr50rdCY35iz+3F9Us8XUmEtNz7/lkq' +
	'TjqFtsf6BzlefNuegLlabA0r3LKDqEtoDm3S4JvLr0VsDS/U/segCX8cP9z6l75Fd1c70fiRsBRiBjXK6/CTsFdgCzjE+IetRGUE' +
	'N9e5mfVHZAckz/iRqFpqE0jeyK/G/hp9TkrW95WpVXkHMYzzl7BQZxMz2+HEr05jAT051evylygMa0O074+jWz4XIs/tka0uE2UI' +
	't5P01U97FwE5nvv51DYKfVGvn7rFIQcUI9v1m6EKAmZLqZPtwT8JVGtMwOCboU9rHR2/lfjaJgFtU6XH/LNUcAkz3//iji87F3RK' +
	'tZnn1n1rDyTK84urOgkmTamC4t0pBHxMgP+XvqhTZ25MutP20zweYFyyjpHsX3UcMMea9d0rSWVHrIn79toifVQlz+SMu08GaVH6' +
	'nuTHIh5iR9/lxrdfeBhAt52L/JgzGXBSrpedr1c8DyujmOjRMRFqAaSB7MM/ZQU1zZyK7c0rHGhIoJaw1ThzES3F55etbhF1UrqN' +
	'+dk5aUInMt/9jb9dbxl0y++IqF9rCzPFroumS2Vmcs//2bsYO1dOQpmm2+tVPUc7wbyZ5wgzB2mNu5yzJn8WM579k70NcXsZJsOm' +
	'mb1acxctw+KFvkl9Fjvd742yVjoFcse1g7lXTmgSJ8/gg6lTcQ4pifDAsR5vFTva+dSxUihJNcflmrfhXXFLOtuQgacBd0pj3+X4' +
	'pSoXYQS6nP7TJxR3UKbK7PqzX21yXaCTuMYiBGNVpIL7zmQECinFme3GOA0sULOUjvnBLQ99FdP4m9giHXNNucX52DoWa1y6luHq' +
	'QWJvSbCzmObUcQFlSKOU4MYgdVA7qIXr2CELcEbrkvTQNwB2Vy/XsvLQNBNtWraO+pwnB2VYr5qbtU1FY0Csj/PMJBtnMEqri+jb' +
	'T2EHUfqY9t43AGNNs4Sk3zkedifd8/LDawsYeVWrkvjGM1thRqXknachHmgHrYnkxDoOYV+vyfPTUEZ/a02onLDWNR16Qr6R7dth' +
	'Hhsy3Ij42DEBK0mki+rU1j8GdxfL4YjEMwVgRrbe/cglBHxArZfm6FlzdFCgkvHs3XEUf1G1iP3RLHlSI6GD+Mo7GH1J4ILlyCoU' +
	'akbbku+Bqx0mUH2at9a7FiMfJZjl2/QPY01pn/CfylNyESrcneKLrV99AiON9sC0UnMJJ96o1eRech85w+GevhdlDSBu3+WLqh13' +
	'CCzP6YW3VWoGJMXtivpLcR8227ub414rERs5xuDPu1l7QDe87YPlUy5fI9mEgaFfaUQ73/iYr1t7YxV+2/aXu0lxHSPV3+DHTHQD' +
	'N9f3gepPZgUp2eWJty5xD21Kqp3po0t9Vj/c/5GtSQpwR+Kf+983BHU/3+nOpb5ffW5Xu4fxmCcDZEOwgYuzSSwHSaGD/ckhH28K' +
	'sJT279lHZwszlpXy3zkFf1Wqm6XaOBt8J9P/maFqDmJJq5eV/sExUW5IqOybo01jdRq6iODEOgJuU6PM8dlYeAM516+dtdY/EnZK' +
	'vZTo33sFCyzY4ZvbPwohQqOO6NIqBX5JF6nnjKJbY2VAtt3+0zsZYUKtkuWDPX8WMMv/+dQiT296ULuN9tkmEj4nzeSIv0sNe0jn' +
	'jeCrSRlqRrqPzL1XfGQTJ6GB9/pfehc1pZn1y10uHzfQ8IezU3sNZsPug6FdWXUHTsvlj+UBMEBvvqWJ8AIyASWY7cHmRjBFYpPj' +
	'j86vWHEDIMHhyr1fbxg8gKTGo1t7FTvM58CzXH8PKcv7jJa5UX4WKc/hh7BcaVN+nPCFp1dgB3zJ7IGwU30TJJqt6YKgCXETI9Si' +
	'xLcHcwInyf+I+FV8FDad7Y+/SDoAJpHYuYvURX0GfoOqmb0gdQcxjPOXsFBnEzPb7cajTmMNPzXV6534RwxrQ7SAj69ZMhciw++d' +
	'rUF/Zgu3kvLTJBR0AzWe+5q5VWURUq+eu8QgBn9Iu5iXoQpjCCvFlunDPApXb0ivjPrLLGsdetHyk94mAm9Wocf73zsQZ1bf/4Xg' +
	'S1IQdkq2muDVfQ1jR6SQ4qtfZ0IroIHi3i0Bf0zmkP3UyzpnCyXduvnWOhhhWraB+IM1G3dZx/+Rt01PZUKoivH22icbNEusjYy3' +
	'TWMPOfuR7cMhF2tHs4Gr0l94FC3X++OVlz8Sf1KvmvHMMlkPK8D2j7dXHmUOq4PsyDUKb17BnOOLokx3A0qtlL/VNhx+RKDlm60I' +
	'exE43Ouf1jAGK05btv2BvVFvGXjJ44ioX2sLM8Wui6ZLaRUhzfOJkwx5EEt/iKfN8Fo0UEaYptbnS2UZP4m/0uwJKVAhkpaCrg1i' +
	'ZF01w7SFr0khDDeC8YT+VW4CZ5minaAFKBkz1bPH9hFYbVI52vbRshoxTHmc7YehTi4Kfcv5xq1Gak1yjaLf7fBBaw1ty/TJs0E6' +
	'ETLevZu+X2BFZNX/maFTY1Mgha6FnrJKKQsr1bPYqUNlE3fRpI/pAmkKJt7tmLdJekw91/H9ibwIegl23faaolZsBj3NoJG7XWcR' +
	'KsXhmuowbA99nICoh6ltawMkzPuPr095UjfC742xTWETK4j7n7hYbxs7I7q+m7ZbeQUx0eGZ8E9rDDTD9/jYLioPKsflmalLewFc' +
	'I8fggLcvD2Q9lvOes1FhCynb7ci3UxscRrOT8KkCZ3IfMcPxka9ZNAtApIXyxyRzBW7L5oerWWENM8XP89Y2Lh5rSMfx2r9WdRkp' +
	'1fmH3WEZfFq6iv+zW21GL8zvgb25PQRxEqqI7cA0Bw8v2b6Vrk9teUummeCNNxN+X6mfm4O5HHcWU7GD9NosHDtArYnzxzN3GyOG' +
	'7+LFIh5mS7eB3eTAIwJzI8/pkfY7HnFZpYnk2i1Kf1u0mIuzXXMJCrmsn/3APRNtW/SL7sxIcwkn04GqzyIPbVGliffBZBt/PCjR' +
	'64enUTofOtf1ibVZZxFs0/ecwxRxH3eW+sTlCRBHRX6EoIP7CDkAL5rj0fZQPFh1hv2Z1FNkW3HHso7lV3lvGGLE4IX2Wn0fL9iv' +
	'x7xUOBw71fuM9AVwET6R6Yu7TAMpSjyK6I+hR3BIf8O8i+dFZxcglr+IokVwEz3T5NyxrkFmGzGi7ZWoCjZVJMueh7NFLgsmy+mV' +
	'oU1zBWDf+5yIsUtnBzGa/5rYOgZ6OcfxzLNXfBIr0/ObrWkMYk6skZb6q104By/G4JuvTwB1H7qP4sA8AWs/yaSXvVR4D1S3rp61' +
	'1jsWdEm9l4O1HG0HLNSM+9o8CidCp4rp1i1pFy1+z+fvzDoObkaw2/7SPx5kQcX3j+RXHHhQppLy0yVPa35QsomdvU11VEOmh+TT' +
	'Jwd/SeKE6cIvdQ0v3+Wo1zMWDXxIqIj4kDkcfzXF95WnUUR5X7iY79s1HWsMpe6DrV9VdQtV8o3qzSUSZkalkbS3QmMPPc3hn8Fu' +
	'EXZRsYbx08VVPhs62fWFsV1jcxalg+TcJREXN8Gqg6hLZRklyfeBusUhBgA3w+OLvRpxEjPR7YmlW21IN9P0nKtffx8l8bTwk/4G' +
	'PVpyn/vD9RVkC37HrcqsADxRYtn1+L9IPQYmw9jqi6VLfEN3wb+JtVl7CzzD6o/7Ty8fMdfg0OtWdB53NdfnkLcLehMwkeONo1Qu' +
	'AT2E892hQ3MEeZPlhKJSdxkPOIbtjPBGLh0/z/jJt1J0DTvb9ZusFnIWatmxiatbbHFJaoqt3a9BZxA5yLGI/lZlBzfAv42kTWsd' +
	'M93zhPRZRGILNtHzg7QMchdg0eKHqV9oATyJoMb9TW8fKJizg/e9T2sFK9yt1PVNOhE52+uc8kRpBy6I/+C5QXpCd57tiey5S31Y' +
	'J8PkjNQgAA85kveCr0FzASHf6cS7MBd0QreU45X+W3YXO8nxnaM6X2NGpp7u2jh3AWrP5oWpWWVmXaz3jurKKxppTMv91r9cfxEt' +
	'po32xmMafl+3hvXXX2lOJT7flOnbNgJ0HqaC5cI3A28zyayHpiIAc0Svnu+KNRN0Ylej54uzFn9zVLyO99MvGSRVs5Tww1N/GSGG' +
	'iu/dKBodc0O73OfdOwgbI83jmZs0AnFEtYn/2Slael+8+IO5V0MRP0W2mPDaLR19VPCO76xYYRs314Wr0SsDe0a5iPfIgUNnDCLb' +
	'44OrOVdwWqyR8s0hBmlo1/+WsEt/Hz+tw+Py1iBhHTHP+dSnTWQIP8vrk8FqD3tPyfWBrVNlTD3X/OSTp0dnEXrf+pe1SXUZJ93f' +
	'wLVcMExplbDRlw4vQH+fkqPA4BRzViiStM3kFSMZO9aagbxQfwA/m7aTulRiBnA8nqKBull1CCvF/43sAmcOOM7t26sONkQmxemU' +
	'vlJrSkE5y+KMull0FyeKvJKxXWAWbsTz16pfdhAmwuaAvh55fmw/zaHQ9gQ4H1rE6J+rS3MFbsvmi6lVYQ0zxaydt1xEcwcnx/Ha' +
	'v1p3FSnV+Ye9Dn8XMNDnk7NbbUYjoYyBvblVZx90x+OErFtvD0C23fvPIG0RLc39heRbfxhXoJT37th0FntU2eWdv091P0CkgPnO' +
	'Ohp6TOeC58hFdQcluIrW7s0qDXpOqYb3nTgSfD3P5erTJkV6XrmW4NI0FGUOq7vzn60yGWdS+YHpwyMUZ0e/g6LCSmkFWqmF+85r' +
	'E3dQSL+L68s3UxM425rl3TEMfQq1m/zcKxV6Us2qgaJDDnFxXqeRtMElDWVSz++PtRB7Ai+inebNOQAtUt/4mK9be28Xctv2m9Yq' +
	'GHQj1bCPq0x0Azfb9Y3qT2oHJdnlibdBHGMHIMD3j6FHfVYz3vORrUllGy2I95O0XGdsbN3lguzoGTZdTJmuxKkMLVUky76H7Qps' +
	'QHyRopm1OH8ILZO2hMuoS2ULPNHp0awfdRk7y/yE/h88W23f8ZegHy8XaNmx9ZenUHBPa4yk0aM8bRUulrvSoUUYDT/JpJu/WHgP' +
	'O9vD9d67VnsVJ939g7UQbwss1OOXtzgNJS/K64e1RWkXIXzD54CgVwxvRrfck7JRfQ0pxfuN6FdzFDykkvTSJUgLEDPd4Z2xT3lU' +
	'K8/o59ImB3xPgumKq0l1AS3T5cC/NBEJeU2t5534U3oXNcn1madRQ35auvCHs1N3D2rD7oOhXVkaZlCYjYOkTHsPI8310rdCbw0x' +
	'p47yxAR7HzjY75e5r1U+Gza0lujbXQ4VcM/rjLRPdRs3wcbjwSwIcyXJ94Hco0dgDDXP4+fQfRh1WdHtiaVbbUg30/iep18Qc0bp' +
	'i5ifvUF9ES/ZtIujSmQfK6SZ6YEhZgsp1eGNs0UgEznQxPOHpy0bWj/a95WpVXkHMYDxm7BQZxMz2+3Go05jAT051euRixRhCGeb' +
	'pf7iHmhQaZLhhfhNIUYmkKXC80Nvbg84hLeM90MpHT/P+M20VncJONv1m6wTchM63OSJq1tsJUs+iPeBr0FnEGCb6Nv9UGUHN8C+' +
	'2qUZakYz3fOE+1lKNAtr0fODtAx8Fz/Q6IfYUWkSap+2haE0YRMlgP+bvKRTZwcn0bqfuld5CznZ55HsU3cQMMfzk7tNJgMuz+OR' +
	'mbVLfTdIrIjg1CAADzmS946tQXENTrCErtQwF3dFtpSMlf5XdBc5qpz3yD5cDyujn+jdOhgBZs3mh8ozDWFcrZzj6MsoH2lPy/HU' +
	'v14ceUWjj/etCBh5XrSD879Ta04nXbKX69RRb3YYooXixTNrBzHJruTMJgR8TqHzhYkyFnZoM8vli7F1GXdevOmV1isYJV64nJCr' +
	'UX8bLemF584tfXl5RrTe66NEYBkjz+/1lD4EYy3RgefWL0gbP9T6g7tbQxk4V9b7maU9G281kO+HrlhjFze7jcqvSmcFOamFl6H8' +
	'T2UMINfjg6tdNhM+0/GNqUV7DWTV/5S8S38fP8mih5K/XWEdMc/1p/hNZFVll6rT7EgqRGGU+ZX4XzsPfZCmvtirUxIdd8Llj6hR' +
	'bxUl3aGM/EpmGijf+YHrWSUVPMvB+YmxFX8YM4DixaNNdV96jqfA+wdtHSWJt4TzQnRUNyHZs42gFmQeftHhmf1WdEN52eCbtU0n' +
	'GDON+oHpRXUNUTje95+tXm8JMZvsgK8ccBYp2eHFqB9oUDTB8+yrDjszRmDN6eSjWTQLL8jon6tLcwVuy+aLqVljATPFoJ+7XERz' +
	'ByfH8dq/WncZK9n5h7EMcxcwv4v/s1ttRiPO74OxuVVrHXiojOnBNgNjL9myl65NYREtwf/miTEVclKllPeDtR53FDfZ5ZHRLh87' +
	'R6KK/sk4Gxchhu2Gp0V5aUO8idvvwyYKfUbD65H0X34TXaKD7dcvRHBXspbk2l9zC27H0pPyxzsdZlX2j+nCJxoLJ9HpzqsqBGNT' +
	'rY//wWkZfVNB0+uFq1E6H1a6n+/cNQB3AbmSkLBLcR87zabvwS4Le31Tppu3wkNkADnD74+5EhJtQqCc4MswDEQ73/Sao1t7YxV+' +
	'vpP01ioUeCPVsI+nTngDN9f3gepPD2JA2eWJt0EcbwUswPeDo0t9VjPe85GtSWUbLYSEwOc=';

const KEY = [
	104, 101, 120, 111, 45, 115, 116, 114, 117, 99, 116, 117, 114, 101, 45, 105, 99, 111, 110, 115
];

let cache: Record<string, Sprite> | null = null;
// .w.
function sprites() {
	if (cache) return cache;
	const raw = atob(BLOB);
	const bytes = new Uint8Array(raw.length);
	for (let i = 0; i < raw.length; i++) {
		bytes[i] = raw.charCodeAt(i) ^ KEY[i % KEY.length] ^ ((i * 31 + 7) & 255);
	}
	cache = JSON.parse(new TextDecoder().decode(bytes)) as Record<string, Sprite>;
	return cache;
}

const SIZE = 16;
const DRAW = 28;
const images = new Map<string, HTMLImageElement>();

function svgOf(set: string, rim: string) {
	const sprite = sprites()[set];
	if (!sprite) return '';
	const colorAt = (x: number, y: number) => {
		if (x < 0 || y < 0 || x >= SIZE || y >= SIZE) return '';
		const ch = sprite.rows[y][x];
		return ch === '.' ? '' : sprite.palette[ALPHABET.indexOf(ch)];
	};
	const rimAt = (x: number, y: number) =>
		!colorAt(x, y) &&
		(colorAt(x - 1, y) || colorAt(x + 1, y) || colorAt(x, y - 1) || colorAt(x, y + 1))
			? rim
			: '';
	const at = (x: number, y: number) => colorAt(x, y) || rimAt(x, y);
	let body = '';
	for (let y = -1; y <= SIZE; y++) {
		let x = -1;
		while (x <= SIZE) {
			const color = at(x, y);
			if (!color) {
				x++;
				continue;
			}
			let end = x + 1;
			while (end <= SIZE && at(end, y) === color) end++;
			body += `<rect x="${x + 1}" y="${y + 1}" width="${end - x}" height="1" fill="${color}"/>`;
			x = end;
		}
	}
	return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${SIZE + 2} ${SIZE + 2}" width="${DRAW}" height="${DRAW}" shape-rendering="crispEdges">${body}</svg>`;
}

export function structureSpriteUrl(set: string, rim: string) {
	const svg = svgOf(set, rim);
	return svg ? `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}` : '';
}

export function structureSprite(set: string, rim: string, onLoad?: () => void) {
	const key = `${set}|${rim}`;
	let img = images.get(key);
	if (!img) {
		const url = structureSpriteUrl(set, rim);
		if (!url) return null;
		img = new Image();
		img.onload = () => onLoad?.();
		img.src = url;
		images.set(key, img);
	}
	return img.complete && img.naturalWidth > 0 ? img : null;
}
