#pragma once

#include <ospf/physics/quantity/dimension/derived_quantity.hpp>

namespace ospf
{
    inline namespace physics
    {
        namespace quantity
        {
			inline namespace derived_quantity
			{
				// Fundamental Quantity
				inline namespace
				{
					//tex:$L$
					using Length = DerivedQuantityType<L_1>;
					//tex:$m$
					using Mass = DerivedQuantityType<M_1>;
					//tex:$t$
					using Time = DerivedQuantityType<T_1>;
					//tex:$I$
					using Current = DerivedQuantityType<I_1>;
					//tex:$\Theta$
					using Temperature = DerivedQuantityType<O_1>;
					//tex:$n$
					using Amount = DerivedQuantityType<N_1>;
					//tex:$l$
					using LuminousIntensity = DerivedQuantityType<J_1>;
					//tex:$rad$
					using PlaneAngle = DerivedQuantityType<R_1>;
					//tex:$sr$
					using SolidAngle = DerivedQuantityType<S_1>;
					//tex:$i$
					using Information = DerivedQuantityType<B_1>;

					static constexpr const auto value = detail::is_derived_quantity<Length>();
				};

				// Basic Derived Quantity
				inline namespace
				{
					//tex:$S = L^{2}$
					using Area = Pow<Length, 2_i64>;
					//tex:$V = L^{3}$
					using Volume = Pow<Length, 3_i64>;
					//tex:$f = t^{-1}$
					using Frequency = Neg<Time>;
				};

				// Mechanics
				inline namespace
				{
					//tex:$v = Lt^{-1}$
					using Velocity = Multiply<Length, Neg<Time>>;
					//tex:$\omega = t^{-1} \cdot rad$
					using AngularVelocity = Multiply<Neg<Time>, PlaneAngle>;
					//tex:$k = L^{-1}$
					using Wavenumber = Neg<Length>;

					//tex:$ a = vt^{-1} $
					using Acceleration = Multiply<Velocity, Neg<Time>>;
					//tex:$\alpha = \omega t^{-1}$
					using AngularAcceleration = Multiply<AngularVelocity, Neg<Time>>;
					//tex:$P = mv$
					using Momentum = Multiply<Mass, Velocity>;
					//tex:$L = m \omega$
					using AngularMomentum = Multiply<Mass, AngularVelocity>;
					//tex:$l = mL^{2}$
					using MomentOfInertia = Multiply<Mass, Pow<Length, 2_i64>>;
					//tex:$F = ma$
					using Force = Multiply<Mass, Acceleration>;
					using Gravity = Force;
					//tex:$p = FS^{-1}$
					using Pressure = Multiply<Force, Neg<Area>>;
					using Stress = Pressure;
					//tex:$I = Ft$
					using Impulse = Multiply<Force, Time>;
					//tex:$\tau = FL$
					using Torque = Multiply<Force, Length>;
					//tex:$\rho = mV^{-1}$
					using MassDensity = Multiply<Mass, Neg<Volume>>;
					//tex:$v = m^{-1}V$
					using SpecificVolume = Multiply<Neg<Mass>, Volume>;

					//tex:$ E = FL $
					using Energy = Multiply<Force, Length>;
					using Work = Energy;
					using Heat = Energy;
					//tex:$c = nV^{-1}$
					using Molarity = Multiply<Amount, Neg<Volume>>;
					//tex:$Vm = n^{-1}V$
					using MolarVolume = Multiply<Neg<Amount>, Volume>;
					//tex:$S = E \Theta^{-1}$
					using Entropy = Multiply<Energy, Neg<Temperature>>;
					//tex:$Cm = Sn^{-1}$
					using MolarEntropy = Multiply<Entropy, Neg<Amount>>;
					using MolarHeatCapacity = MolarEntropy;
					//tex:$c = Sm^{-1}$
					using SpecificEntropy = Multiply<Entropy, Neg<Mass>>;
					using SpecificHeatCapacity = SpecificEntropy;
					//tex:$Em = En^{-1}$
					using MolarEnergy = Multiply<Energy, Neg<Amount>>;
					//tex:$h = Em^{-1}$
					using SpecificEnergy = Multiply<Energy, Neg<Mass>>;
					//tex:$U = EV^{-1}$
					using EnergyDensity = Multiply<Energy, Neg<Volume>>;
					//tex:$Cm = E \Theta^{-1}$
					using HeatCapacity = Multiply<Energy, Neg<Temperature>>;
					//tex:$\sigma = FL^{-1}$
					using SurfaceTension = Multiply<Force, Neg<Length>>;
					//tex:$P = Fv$
					using Power = Multiply<Force, Velocity>;
					//tex:$E = PS^{-1}$
					using PowerDensity = Multiply<Power, Neg<Area>>;
					using Irradiance = PowerDensity;
					using HeatFluxDensity = PowerDensity;
					//tex:$\lambda = P(L\Theta)^{-1}$
					using ThermalConductivity = Multiply<Power, Neg<Multiply<Length, Temperature>>>;
					//tex:$\mu = pt$
					using DynamicViscosity = Multiply<Pressure, Time>;
					//tex:$\gamma = \mu \rho^{-1}$
					using KinematicViscosity = Multiply<DynamicViscosity, Neg<MassDensity>>;
					//tex:$M = mn^{-1}$
					using MolarMass = Multiply<Mass, Neg<Amount>>;

					//tex:$mL^{-1}$
					using LinearDensity = Multiply<Mass, Neg<Length>>;
					//tex:$mS^{-1}$
					using SurfaceDensity = Multiply<Mass, Neg<Area>>;
					//tex:$Et$
					using Action = Multiply<Energy, Time>;
				};

				// Electromagnetism
				inline namespace
				{
					//tex:$ Q = It $
					using ElectricCharge = Multiply<Current, Time>;
					//tex:$ \rho = QV^{-1} $
					using ElectricChargeDensity = Multiply<ElectricCharge, Neg<Volume>>;
					//tex:$J = IS^{-1}$
					using CurrentDensity = Multiply<ElectricCharge, Neg<Volume>>;
					//tex:$U = PI^{-1}$
					using ElectricPotential = Multiply<Power, Neg<Current>>;
					using ElectromotiveForce = ElectricPotential;
					using Voltage = ElectricPotential;
					//tex:$R = UI^{-1}$
					using Resistance = Multiply<Voltage, Current>;
					using Impedance = Resistance;
					//tex:$G = R^{-1}$
					using Conductance = Neg<Resistance>;
					//tex:$\kappa = GL^{-1}$
					using Conductivity = Multiply<Conductance, Neg<Length>>;
					//tex:$ \kappa m = \kappa \cdot Vm $
					using MolarConductivity = Multiply<Conductivity, MolarVolume>;
					//tex:$F = QU^{-1}$
					using Capacitance = Multiply<ElectricCharge, Neg<Voltage>>;
					//tex:$\epsilon = FL^{-1}$
					using Permittivity = Multiply<Capacitance, Neg<Length>>;
					//tex:$E = UL^{-1} $
					using ElectricFieldIntensity = Multiply<ElectricPotential, Neg<Length>>;
					using ElectricFieldStrength = ElectricFieldIntensity;
					//tex:$L = UI^{-1}t$
					using Inductance = Multiply<ElectricPotential, Multiply<Neg<Current>, Time>>;
					//tex:$B = F(LI)^{-1}$
					using MagneticFieldDensity = Multiply<Force, Neg<Multiply<Length, Current>>>;
					//tex:$H = IL^{-1} $
					using MagneticFieldIntensity = Multiply<Current, Neg<Length>>;
					//tex:$\phi = BS$
					using MagneticFlux = Multiply<MagneticFieldDensity, Area>;
					//tex:$\mu = LL^{-1}$
					using MagneticPermeability = Multiply<Inductance, Neg<Length>>;
					//tex:$\lambda = \mu^{-1}$
					using MagneticReluctance = Neg<MagneticPermeability>;

					//tex:$ QL^{-1} $
					using ElectricChargeLinearDensity = Multiply<ElectricCharge, Neg<Length>>;
					//tex:$ QS^{-1} $
					using ElectricChargeSurfaceDensity = Multiply<ElectricCharge, Neg<Area>>;
				};

				// Optics
				inline namespace
				{
					//tex:$\phi = l \cdot sr$
					using LuminousFlux = Multiply<LuminousIntensity, SolidAngle>;
					//tex:$\iota = \phi S^{-1}$
					using Illuminace = Multiply<LuminousFlux, Neg<Area>>;
					//tex:$L = lS^{-1}$
					using Luminance = Multiply<LuminousIntensity, Neg<Area>>;
				};

				// Radiology
				inline namespace
				{
					//tex:$A = t^{-1}$
					using Activity = Neg<Time>;
					//tex:$D = Em^{-1}$
					using AbsorbedDose = Multiply<Energy, Neg<Mass>>;
					using DoseEquivalent = AbsorbedDose;
					//tex:$d = Dt^{-1}$
					using DosingRate = Multiply<DoseEquivalent, Neg<Time>>;
				};

				// Ohter
				inline namespace
				{
					//tex:$z = nt^{-1}$
					using CatalyticActivity = Multiply<Amount, Neg<Time>>;
					//tex:$it^{-1}$
					using Bandwidth = Multiply<Information, Neg<Time>>;
				};
			};
        };
    };
};
