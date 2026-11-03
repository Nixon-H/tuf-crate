        Method (WMNB, 3, Serialized)
        {
            P8XH (Zero, 0x11)
            CreateDWordField (Arg2, Zero, IIA0)
            CreateDWordField (Arg2, 0x04, IIA1)
            CreateDWordField (Arg2, 0x08, IIA2)
            CreateDWordField (Arg2, 0x0C, IIA3)
            CreateDWordField (Arg2, 0x10, IIA4)
            Local0 = (Arg1 & 0xFFFFFFFF)
            If ((Local0 == 0x54494E49))
            {
                INIT (IIA0)
                Return (One)
            }

            If ((Local0 == 0x53545342))
            {
                Return (BSTS ())
            }

            If ((Local0 == 0x4E554653))
            {
                Return (SFUN ())
            }

            If ((Local0 == 0x47444353))
            {
                Return (SCDG (IIA0, IIA1))
            }

            If ((Local0 == 0x53524556))
            {
                Return (VERS (IIA0, IIA1))
            }

            If ((Local0 == 0x4647574D))
            {
                Return (MWGF (IIA0, IIA1, Local0))
            }

            If ((Local0 == 0x44434C47))
            {
                Return (GLCD ())
            }

            If ((Local0 == 0x474F4457))
            {
                Return (WDOG (IIA0))
            }

            If ((Local0 == 0x5256534F))
            {
                Return (OSVR (IIA0))
            }

            If ((Local0 == 0x49564E41))
            {
                Return (ANVI (IIA0))
            }

            If ((Local0 == 0x53545344))
            {
                If ((IIA0 == 0x00130031))
                {
                    WNVA = 0x9601
                    WNVB = 0x00130031
                    WNVC = IIA1 /* \_SB_.ATKD.WMNB.IIA1 */
                    WNVD = Zero
                    Return (W15H ())
                }

                If ((IIA0 == 0x00060024))
                {
                    Return (Package (0x03)
                    {
                        0x06ED8086, 
                        One, 
                        0xFFFFFFFF
                    })
                }

                If ((IIA0 == 0x00060025))
                {
                    Return (0xFFFFFFFE)
                    Return (Package (0x02)
                    {
                        0x1400, 
                        0xFFFFFFFF
                    })
                }

                If ((IIA0 == 0x00100051))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00130021))
                {
                    Local0 = 0x00010000
                    Local0 |= 0x04
                    Return (Local0)
                }

                If ((IIA0 == 0x00130022))
                {
                    WNVA = 0x9601
                    WNVB = 0x00130022
                    WNVC = Zero
                    WNVD = Zero
                    Return (W15H ())
                }

                If ((IIA0 == 0xA0000001))
                {
                    Return (0x02)
                }

                If ((IIA0 == 0x00040017))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00040019))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x0004001A))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050001))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050013))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050031))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050032))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050033))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050035))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050043))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050051))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00060023))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00060026))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00060059))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x0006005C))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x0006005D))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00060061))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00060078))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00080041))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00080042))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00080043))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00090014))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00100043))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00100053))
                {
                    Return (0xFFFFFFFE)
                    Return (Zero)
                }

                If ((IIA0 == 0x00110011))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x0011001E))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120058))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120061))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120063))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120065))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120082))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120083))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120087))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120088))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00010031))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x0005001A))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x0005001E))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x0005001F))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050024))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050025))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050026))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00030022))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050036))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x0005003A))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x0005003B))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x0006005B))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00060064))
                {
                    Return (0xFFFFFFFE)
                    Return (Zero)
                }

                If ((IIA0 == 0x0006006A))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x0006006B))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00060091))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00060093))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00060094))
                {
                    Return (0x00010002)
                }

                If ((IIA0 == 0x00090014))
                {
                    Return (0xFFFFFFFE)
                    Return (Zero)
                }

                If ((IIA0 == 0x0009001A))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00100052))
                {
                    Return (0xFFFFFFFE)
                    Return (Zero)
                }

                If ((IIA0 == 0x00100054))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x0010005A))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00100066))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00110013))
                {
                    If (^^PC00.LPCB.EC0.ECOK)
                    {
                        Acquire (^^PC00.LPCB.EC0.MUT0, 0x1000)
                        Local0 = ^^PC00.LPCB.EC0.FRMS /* \_SB_.PC00.LPCB.EC0_.FRMS */
                        Release (^^PC00.LPCB.EC0.MUT0)
                        If ((Local0 != Zero))
                        {
                            Divide (0x0020E6DA, Local0, Local1, Local0)
                            Divide (Local0, 0x64, Local1, Local0)
                        }

                        Local0 |= 0x00010000
                        Return (Local0)
                    }
                }

                If ((IIA0 == 0x00110014))
                {
                    If (^^PC00.LPCB.EC0.ECOK)
                    {
                        Acquire (^^PC00.LPCB.EC0.MUT0, 0x1000)
                        Local0 = ^^PC00.LPCB.EC0.FRS2 /* \_SB_.PC00.LPCB.EC0_.FRS2 */
                        Release (^^PC00.LPCB.EC0.MUT0)
                        If ((Local0 != Zero))
                        {
                            Divide (0x0020E6DA, Local0, Local1, Local0)
                            Divide (Local0, 0x64, Local1, Local0)
                        }

                        Local0 |= 0x00010000
                        Return (Local0)
                    }
                }

                If ((IIA0 == 0x00110015))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00110016))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00110018))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00110021))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120058))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120067))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120072))
                {
                    Return (CRFC ())
                }

                If ((IIA0 == 0x00120081))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120089))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120090))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120094))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00130041))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x0005001C))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x0005001D))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00060077))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00060095))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00090017))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00090018))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00090019))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x0009001C))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00090021))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x0010005B))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x0011001D))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x0011001B))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00110024))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00110025))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00110026))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00110027))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00110028))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00110029))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00110031))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00110032))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00110033))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x0012006C))
                {
                    Acquire (^^PC00.LPCB.EC0.MUT0, 0x0500)
                    Local0 = ^^PC00.LPCB.EC0.ACIN /* \_SB_.PC00.LPCB.EC0_.ACIN */
                    Release (^^PC00.LPCB.EC0.MUT0)
                    If ((Local0 == Zero))
                    {
                        Return (0x00010000)
                    }
                    Else
                    {
                        Return (0x00010001)
                    }
                }

                If ((IIA0 == 0x00120076))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120077))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120079))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x0012007B))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120091))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00120092))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00120093))
                {
                    Local0 = ^^NPCF.AMAT /* External reference */
                    Local0 /= 0x08
                    Local0 += 0x37
                    Return (Local0)
                }

                If ((IIA0 == 0x00120095))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120096))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120097))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x001200A0))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x001200A1))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x001200A2))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x001200A3))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x001200B0))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x001200B1))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x001200C0))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x001200C1))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x001200C2))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x001200C3))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x001200D1))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00130023))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00090020))
                {
                    Local0 = 0x00010000
                    Local0 |= CUMA /* \CUMA */
                    Return (Local0)
                }

                If ((IIA0 == 0x00090030))
                {
                    Local1 = 0x00010000
                    If (((^^PC00.LPCB.EC0.DPHP == One) || (^^PC00.LPCB.EC0.TBTI == One)))
                    {
                        Local1 = 0x00010044
                        Return (Local1)
                    }
                    Else
                    {
                        Local1 = 0x00010040
                        Return (Local1)
                    }
                }

                If ((IIA0 == 0x00090016))
                {
                    Return (DGPM) /* \DGPM */
                }

                If ((IIA0 == 0x00130026))
                {
                    Return (0x00010001)
                }

                If (WCHK (Zero, IIA0))
                {
                    WNVA = 0x9603
                    WNVB = 0x02
                    WNVC = IIA0 /* \_SB_.ATKD.WMNB.IIA0 */
                    AII0 = IIA0 /* \_SB_.ATKD.WMNB.IIA0 */
                    AII1 = IIA1 /* \_SB_.ATKD.WMNB.IIA1 */
                    AII2 = IIA2 /* \_SB_.ATKD.WMNB.IIA2 */
                    AII3 = IIA3 /* \_SB_.ATKD.WMNB.IIA3 */
                    AII4 = IIA4 /* \_SB_.ATKD.WMNB.IIA4 */
                    Return (W15H ())
                }

                Return (0xFFFFFFFE)
            }

            If ((Local0 == 0x53564544))
            {
                If ((IIA0 == 0x00050011))
                {
                    If ((IIA1 == 0x02))
                    {
                        BLCT = One
                    }
                    Else
                    {
                        BLCT = Zero
                    }
                }

                If ((IIA0 == 0x00100021))
                {
                    Return (^^PC00.LPCB.EC0.SKEY (IIA1))
                }

                If ((IIA0 == 0x00130022))
                {
                    WNVA = 0x9602
                    WNVB = 0x00130022
                    WNVC = IIA1 /* \_SB_.ATKD.WMNB.IIA1 */
                    WNVD = Zero
                    Return (W15H ())
                }

                If ((IIA0 == 0x00040017))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00040019))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050001))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050013))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050031))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050032))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050035))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00060023))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00060026))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00060078))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00090014))
                {
                    Return (0xFFFFFFFE)
                    Return (Zero)
                }

                If ((IIA0 == 0x00100066))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x0011001E))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120058))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00010031))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x0005001A))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050024))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050025))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050026))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00030022))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00050036))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00060064))
                {
                    Return (0xFFFFFFFE)
                    Return (Zero)
                }

                If ((IIA0 == 0x00060091))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00090040))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00090016))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00100052))
                {
                    Return (0xFFFFFFFE)
                    Return (Zero)
                }

                If ((IIA0 == 0x00100054))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00110013))
                {
                    If (^^PC00.LPCB.EC0.ECOK)
                    {
                        Acquire (^^PC00.LPCB.EC0.MUT0, 0x1000)
                        Local0 = ^^PC00.LPCB.EC0.FTBL /* \_SB_.PC00.LPCB.EC0_.FTBL */
                        Release (^^PC00.LPCB.EC0.MUT0)
                        If ((Local0 == Zero))
                        {
                            Arg1 = Zero
                        }

                        Arg1 = One
                    }

                    Return (One)
                }

                If ((IIA0 == 0x00110014))
                {
                    If (^^PC00.LPCB.EC0.ECOK)
                    {
                        Acquire (^^PC00.LPCB.EC0.MUT0, 0x1000)
                        Local0 = ^^PC00.LPCB.EC0.FTBL /* \_SB_.PC00.LPCB.EC0_.FTBL */
                        Release (^^PC00.LPCB.EC0.MUT0)
                        If ((Local0 == Zero))
                        {
                            Arg1 = Zero
                        }

                        Arg1 = One
                    }

                    Return (One)
                }

                If ((IIA0 == 0x00110015))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00110016))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00110018))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00110021))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120072))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120073))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120074))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120000))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120089))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00130025))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0xC0400000))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0xC0400001))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x0005001C))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x0005001D))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00090019))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00110024))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00110025))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00110028))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00110032))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00120076))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00120092))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00120095))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x001200A0))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x001200A1))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x001200A2))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x001200A3))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x001200B0))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x001200B1))
                {
                    Return (0xFFFFFFFE)
                }

                If ((IIA0 == 0x00060095))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x0012007B))
                {
                    Return (Zero)
                }

                If ((IIA0 == 0x00090020))
                {
                    CUMA = IIA1 /* \_SB_.ATKD.WMNB.IIA1 */
                    If ((IIA1 == One))
                    {
                        WOSR = One
                        If ((^^PC00.LPCB.EC0.UMAF != One))
                        {
                            If ((^^PC00.LPCB.EC0.FTBL == 0x02))
                            {
                                ODV0 = 0x03
                                Notify (^^PC00.TCPU, 0x83) // Device-Specific Change
                                Notify (IETM, 0x85) // Device-Specific
                                Notify (IETM, 0x88) // Device-Specific
                            }

                            Notify (^^PC00.PEG1.PEGP, 0x81) // Information Change
                            ^^PC00.LPCB.EC0.UMAF = One
                            Notify (^^PC00.PEG1.PEGP, 0x03) // Eject Request
                            If ((^^PC00.LPCB.EC0.WAT1 () == One))
                            {
                                Return (Zero)
                            }
                        }

                        If ((^^PC00.LPCB.EC0.UMAF == One))
                        {
                            ^^PC00.LPCB.EC0.UMAF = One
                        }
                    }
                    Else
                    {
                        If ((^^PC00.LPCB.EC0.UMCF == One))
                        {
                            ^^PC00.LPCB.EC0.UMCF = Zero
                            ^^PC00.LPCB.EC0.UMOE = Zero
                            ^^PC00.LPCB.EC0.UMAF = Zero
                            UMFG = Zero
                            Notify (^^PC00.PEG1.PEGP, One) // Device Check
                            Notify (^^PC00.PEG1.PEGP, 0x80) // Status Change
                            Notify (^^PC00.PEG1.PEGP, 0x81) // Information Change
                            Notify (^^PC00.PEG1.PEGP, 0x82) // Device-Specific Change
                            ^^PC00.LPCB.EC0.NVON ()
                            Notify (^^PC00.PEG1.PEGP, Zero) // Bus Check
                            Notify (^^PC00.PEG1.PEGP, One) // Device Check
                            Notify (^^PC00.PEG1.PEGP, 0x80) // Status Change
                            Notify (^^PC00.PEG1.PEGP, 0x81) // Information Change
                            Notify (^^PC00.PEG1.PEGP, 0x82) // Device-Specific Change
                            ^^PC00.LPCB.EC0.FGA9 = One
                            Sleep (0x2328)
                            Notify (^^PC00.PEG1.PEGP, One) // Device Check
                            Notify (^^PC00.PEG1.PEGP, 0x80) // Status Change
                            Notify (^^PC00.PEG1.PEGP, 0x81) // Information Change
                            Notify (^^PC00.PEG1.PEGP, 0x82) // Device-Specific Change
                            If ((^^PC00.LPCB.EC0.WAT2 () == One))
                            {
                                Return (Zero)
                            }
                        }

                        If ((^^PC00.LPCB.EC0.UMCF != One))
                        {
                            If ((^^PC00.LPCB.EC0.FTBL == 0x02))
                            {
                                ODV0 = One
                                Notify (^^PC00.TCPU, 0x83) // Device-Specific Change
                                Notify (IETM, 0x85) // Device-Specific
                                Notify (IETM, 0x88) // Device-Specific
                            }

                            UMFG = Zero
                            ^^PC00.LPCB.EC0.UMAF = Zero
                            Notify (^^PC00.PEG1.PEGP, Zero) // Bus Check
                            Sleep (0x2328)
                            Notify (^^PC00.PEG1.PEGP, One) // Device Check
                            Notify (^^PC00.PEG1.PEGP, 0x80) // Status Change
                            Notify (^^PC00.PEG1.PEGP, 0x81) // Information Change
                            Notify (^^PC00.PEG1.PEGP, 0x82) // Device-Specific Change
                            ^^PC00.LPCB.EC0.FGA9 = One
                            Notify (^^PC00.PEG1.PEGP, One) // Device Check
                            Notify (^^PC00.PEG1.PEGP, 0x80) // Status Change
                            Notify (^^PC00.PEG1.PEGP, 0x81) // Information Change
                            Notify (^^PC00.PEG1.PEGP, 0x82) // Device-Specific Change
                            If ((^^PC00.LPCB.EC0.WAT2 () == One))
                            {
                                Return (Zero)
                            }
                        }
                    }

                    Sleep (0x07D0)
                    While ((^^PC00.LPCB.EC0.NVPS == One))
                    {
                        Sleep (0x64)
                    }

                    Return (One)
                }

                If (WCHK (One, IIA0))
                {
                    WNVA = 0x9603
                    WNVB = 0x03
                    WNVC = IIA0 /* \_SB_.ATKD.WMNB.IIA0 */
                    AII0 = IIA0 /* \_SB_.ATKD.WMNB.IIA0 */
                    AII1 = IIA1 /* \_SB_.ATKD.WMNB.IIA1 */
                    AII2 = IIA2 /* \_SB_.ATKD.WMNB.IIA2 */
                    AII3 = IIA3 /* \_SB_.ATKD.WMNB.IIA3 */
                    AII4 = IIA4 /* \_SB_.ATKD.WMNB.IIA4 */
                    Return (W15H ())
                }

                Return (0xFFFFFFFE)
            }

            Return (0xFFFFFFFE)
        }