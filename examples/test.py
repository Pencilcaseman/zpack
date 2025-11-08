import zpack
from zpack.constraint import *
from zpack.package import Version, PackageOutline


zpack.init_tracing()


class Hpl:
    def outline(self):
        constraints = [
            Depends("blas"),
            Depends("mpi"),
            Depends("c-compiler"),
        ]

        outline = PackageOutline("hpl")
        outline.push_constraints(constraints)

        return outline


class Blas:
    def outline(self):
        constraints = [
            SpecOption("blas", "openblas").if_then(Depends("openblas")),
            SpecOption("blas", "mkl").if_then(Depends("mkl")),

            NumOf([
                SpecOption("blas", "openblas"),
                SpecOption("blas", "mkl"),
            ]) == 1
        ]

        outline = PackageOutline("blas")
        outline.push_constraints(constraints)

        return outline


class Mpi:
    def outline(self):
        constraints = [
            SpecOption("mpi", "openmpi").if_then(Depends("openmpi")),
            SpecOption("mpi", "mpich").if_then(Depends("mpich")),
            SpecOption("mpi", "intelmpi").if_then(Depends("intelmpi")),

            NumOf([
                SpecOption("mpi", "openmpi"),
                SpecOption("mpi", "mpich"),
                SpecOption("mpi", "intelmpi"),
            ]) == 1
        ]

        outline = PackageOutline("mpi")
        outline.push_constraints(constraints)

        return outline


class OpenBlas:
    def outline(self):
        constraints = [
            Depends("c-compiler")
        ]

        outline = PackageOutline("openblas")
        outline.push_constraints(constraints)

        return outline


class Mkl:
    def outline(self):
        constraints = [
            Depends("c-compiler")
        ]

        outline = PackageOutline("mkl")
        outline.push_constraints(constraints)

        return outline


class OpenMpi:
    def __init__(self):
        self.versions = [
            "5.0.8",
            "5.0.7",
            "5.0.6",
            # "5.0.5",
            # "5.0.4",
            # "5.0.3",
            # "5.0.2",
            # "5.0.1",
            # "5.0.0",
            # "4.1.8",
            # "4.1.7",
            # "4.1.6",
            # "4.1.5",
            # "4.1.4",
            # "4.1.3",
            # "4.1.2",
            # "4.1.1",
            # "4.1.0",
            # "1.2.3",
            # "1.2.3.4.5.6.7.8.9.10",
        ]

    def outline(self):
        outline = PackageOutline("openmpi")

        version_option = SpecOption("openmpi", "version")

        version_constraints = [
            version_option == Version(v) for v in self.versions
        ]

        constraints = [
            NumOf(version_constraints) == 1,

            Depends("openpmix"),
            Depends("openprrte"),
            Depends("hwloc"),
            Depends("c-compiler"),
        ]

        print(constraints)

        outline.push_constraints(constraints)

        return outline


class Mpich:
    def outline(self):
        constraints = [
            Depends("c-compiler")
        ]

        outline = PackageOutline("mpich")
        outline.push_constraints(constraints)

        return outline


class IntelMpi:
    def outline(self):
        constraints = [
            Depends("c-compiler")
        ]

        outline = PackageOutline("intelmpi")
        outline.push_constraints(constraints)

        return outline


class OpenPmix:
    def outline(self):
        constraints = [
            Depends("c-compiler")
        ]

        outline = PackageOutline("openpmix")
        outline.push_constraints(constraints)

        return outline


class OpenPrrte:
    def outline(self):
        constraints = [
            Depends("c-compiler")
        ]

        outline = PackageOutline("openprrte")
        outline.push_constraints(constraints)

        return outline


class HwLoc:
    def outline(self):
        constraints = [
            Depends("c-compiler")
        ]

        outline = PackageOutline("hwloc")
        outline.push_constraints(constraints)

        return outline


class CCompiler:
    def outline(self):
        outline = PackageOutline("c-compiler")
        return outline


def zpack_packages():
    return [
        Hpl(),
        Blas(),
        Mpi(),
        OpenBlas(),
        Mkl(),
        OpenMpi(),
        IntelMpi(),
        Mpich(),
        OpenPmix(),
        OpenPrrte(),
        HwLoc(),
        CCompiler(),
    ]
